#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

/// One of the simplest smart contracts possible,
/// it holds a single variable in storage, which anyone can increment.
#[elrond_wasm::contract]
pub trait Adder {
    #[init]
    fn init(&self, stake_token: TokenIdentifier) {
        self.stake_token().set_if_empty(&stake_token);
        self.origin_epoch().set_if_empty(&self.blockchain().get_block_epoch());
    }

    #[only_owner]
    #[endpoint(depositReward)]
    fn deposit_reward(&self) -> BigUint {
        let current_epoch = self.blockchain().get_block_epoch();
        let payment = self.call_value().egld_or_single_esdt();
        require!(&payment.amount > &BigUint::zero(), "No payment sent");

        let payment_token_name = payment.token_identifier;
        let mut reward_stats;
        if self.reward_payment_info(&payment_token_name).is_empty() {
            reward_stats = RewardPaymentInfo {
                last_paid_epoch: self.origin_epoch().get(),
                total_paid_so_far: BigUint::zero(),
                last_reward_payment: BigUint::zero(),
                last_reward_per_epoch_per_nonce: BigUint::zero(),
            };
        } else {
            reward_stats = self.reward_payment_info(&payment_token_name).get();
        }

        let mut total_claimable_epochs = 0u64;
        for address in self.staked_nfts().keys() {
            total_claimable_epochs += self.get_total_payable_epochs(current_epoch, reward_stats.last_paid_epoch, &address);
        }

        let reward_per_epoch_per_nonce = &payment.amount / &BigUint::from(total_claimable_epochs);
        for address in self.staked_nfts().keys() {
            let payable_epochs = self.get_total_payable_epochs(current_epoch, reward_stats.last_paid_epoch, &address);
            let claimable_amount = &BigUint::from(payable_epochs) * &reward_per_epoch_per_nonce;
            let payment_info = EgldOrEsdtTokenPayment::new(payment_token_name.clone(), payment.token_nonce, claimable_amount);
            self.claimable_rewards(&address).push(&payment_info);
        }

        reward_stats.last_paid_epoch = current_epoch;
        reward_stats.total_paid_so_far = &reward_stats.total_paid_so_far + &payment.amount;
        reward_stats.last_reward_payment = payment.amount;
        reward_stats.last_reward_per_epoch_per_nonce = reward_per_epoch_per_nonce.clone();

        self.reward_payment_info(&payment_token_name).set(&reward_stats);

        return reward_per_epoch_per_nonce;
    }

    #[payable("*")]
    #[endpoint(stake)]
    fn stake(&self) {
        let payment = self.call_value().all_esdt_transfers();
        require!(payment.len() > 0, "cannot stake nothing");
        let stake_token = self.stake_token().get();
        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();

        let mut existing_stake;
        if !self.staked_nfts().contains_key(&caller) {
            existing_stake = ManagedVec::new();
        } else {
            existing_stake = self.staked_nfts().remove(&caller).unwrap();
        }
        for nft in payment.iter() {
            require!(nft.token_identifier == stake_token, "one or more NFTs is not eligible for staking");
            let staked_nft_info = StakedNftInfo {
                nonce: nft.token_nonce,
                stake_epoch: current_epoch,
            };
            existing_stake.push(staked_nft_info);
        }

        self.staked_nfts().insert(caller, existing_stake);
    }

    #[endpoint(unstake)]
    fn unstake(&self, nonces_to_unstake_vec: ManagedVec<u64>) {
        let caller = self.blockchain().get_caller();
        require!(self.staked_nfts().contains_key(&caller), "nothing to unstake");
        let stake_token = self.stake_token().get();
        // let nonces_to_unstake_vec = nonces_to_unstake.to_vec();

        let staked_nfts = self.staked_nfts().remove(&caller).unwrap();
        let mut remaining_stake = ManagedVec::new();
        for nft in staked_nfts.iter() {
            if nonces_to_unstake_vec.contains(&nft.nonce) {
                self.send().direct_esdt(
                    &caller,
                    &stake_token,
                    nft.nonce,
                    &BigUint::from(1u32)
                );
            } else {
                remaining_stake.push(nft);
            }
        }
        if !remaining_stake.is_empty() {
            self.staked_nfts().insert(caller, remaining_stake);
        }
    }

    #[endpoint(claimReward)]
    fn claim_reward(&self) {
        let caller = self.blockchain().get_caller();
        require!(!self.claimable_rewards(&caller).is_empty(), "no rewards to claim");
        for reward in self.claimable_rewards(&caller).iter() {
            self.send().direct(
                &caller,
                &reward.token_identifier,
                reward.token_nonce,
                &reward.amount
            );
        }
        self.claimable_rewards(&caller).clear();
    }

    #[view(getTotalPayableEpochs)]
    fn get_total_payable_epochs(&self, current_epoch: u64, last_reward_epoch: u64, address: &ManagedAddress) -> u64 {
        if !self.staked_nfts().contains_key(address) {
            return 0u64;
        }
        let mut payable_epochs = 0u64;
        let staked_nfts = self.staked_nfts().get(address).unwrap();
        for nft in staked_nfts.iter() {
            // can only be staked as follows stake_epoch >= last_reward_epoch in the first month
            // in the second month, NFTs were staked as follows stake_epoch < last_reward_epoch
            
            if last_reward_epoch > nft.stake_epoch { // 20 > 15 sau 20 > 0
                // staked before the current reward round
                payable_epochs += current_epoch - last_reward_epoch; // full period of time
                continue;
            }
            payable_epochs += current_epoch - nft.stake_epoch; 
            // 140 fara check-ul de mai sus
            // 125 cu check
            // expected e 100
        }
        return payable_epochs;
    }

    #[view(getStakedNfts)]
    fn get_staked_nfts(&self, address: ManagedAddress) -> ManagedVec<StakedNftInfo> {
        if !self.staked_nfts().contains_key(&address) {
            return ManagedVec::new();
        }
        return self.staked_nfts().get(&address).unwrap();
    }

    #[view(getOriginEpoch)]
    fn get_origin_epoch(&self) -> u64 {
        return self.origin_epoch().get();
    }

    #[view(getStakeToken)]
    fn get_stake_token(&self) -> TokenIdentifier {
        return self.stake_token().get();
    }

    #[view(getRewardPaymentInfo)]
    fn get_reward_payment_info(&self, token: EgldOrEsdtTokenIdentifier) -> RewardPaymentInfo<Self::Api> {
        return self.reward_payment_info(&token).get();
    }

    #[storage_mapper("staked_nfts")]
    fn staked_nfts(&self) -> MapMapper<ManagedAddress, ManagedVec<StakedNftInfo>>;

    #[storage_mapper("origin_epoch")]
    fn origin_epoch(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("stake_token")]
    fn stake_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("reward_payment_info")]
    fn reward_payment_info(&self, reward_token: &EgldOrEsdtTokenIdentifier) -> SingleValueMapper<RewardPaymentInfo<Self::Api>>;

    #[view(getClaimableRewards)]
    #[storage_mapper("claimable_rewards")]
    fn claimable_rewards(&self, address: &ManagedAddress) -> VecMapper<EgldOrEsdtTokenPayment>;
}

#[derive(ManagedVecItem, NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, Clone, PartialEq)]
pub struct RewardPaymentInfo<M: ManagedTypeApi> {
    pub last_paid_epoch: u64,
    pub total_paid_so_far: BigUint<M>,
    pub last_reward_payment: BigUint<M>,
    pub last_reward_per_epoch_per_nonce: BigUint<M>,
}

#[derive(ManagedVecItem, NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, Clone, PartialEq)]
pub struct StakedNftInfo {
    pub nonce: u64,
    pub stake_epoch: u64
}