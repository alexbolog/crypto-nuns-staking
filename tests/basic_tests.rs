use cnuns_staking::*;
use elrond_wasm::elrond_codec::multi_types::OptionalValue;
use elrond_wasm::types::{Address, BoxedBytes, ManagedVec, ManagedBuffer};
use elrond_wasm_debug::{
    managed_biguint, rust_biguint, testing_framework::*, num_bigint, managed_token_id, managed_address,
    DebugApi,
};
use elrond_wasm_debug::tx_mock::TxInputESDT;

const WASM_PATH: &'static str = "../output/cnuns_staking.wasm";
const STAKE_TOKEN: &[u8] = b"CNUN-123456";
const INVALID_STAKE_TOKEN: &[u8] = b"RANDOM-123456";
const REWARD_TOKEN: &[u8] = b"REW-abcdef";
const REWARD_AMOUNT: u64 = 1_000_000_000;

struct StakingSetup<StakingObjBuilder>
where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub staking_sc_wrapper:
        ContractObjWrapper<cnuns_staking::ContractObj<DebugApi>, StakingObjBuilder>,
    pub owner_address: Address,
    pub client_address: Address,
    pub client2_address: Address,
    pub client3_address: Address,
}

fn setup_staking<StakingObjBuilder>(
    cf_builder: StakingObjBuilder,
) -> StakingSetup<StakingObjBuilder>
where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);

    let mut blockchain_wrapper = BlockchainStateWrapper::new();

    let owner_address = blockchain_wrapper.create_user_account(&rust_zero);
    let client_address = blockchain_wrapper.create_user_account(&rust_zero);
    let client2_address = blockchain_wrapper.create_user_account(&rust_zero);
    let client3_address = blockchain_wrapper.create_user_account(&rust_zero);

    let staking_sc_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        cf_builder,
        WASM_PATH,
    );

    // deploy
    blockchain_wrapper
        .execute_tx(&owner_address, &staking_sc_wrapper, &rust_zero, |sc| {
            let token_identifier = managed_token_id!(STAKE_TOKEN);
            sc.init(
                OptionalValue::Some(token_identifier)  
            );
        })
        .assert_ok();

    let reward_amount = num_bigint::ToBigUint::to_biguint(&(10 * REWARD_AMOUNT)).unwrap();
    blockchain_wrapper.set_egld_balance(&owner_address, &reward_amount);
    blockchain_wrapper.set_esdt_balance(&owner_address, REWARD_TOKEN, &reward_amount);
    
    let nft_balance = num_bigint::ToBigUint::to_biguint(&1).unwrap();
    blockchain_wrapper.set_nft_balance(&client_address, INVALID_STAKE_TOKEN, 1, &nft_balance, &BoxedBytes::empty());

    blockchain_wrapper.set_nft_balance(&client_address, STAKE_TOKEN, 1, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client_address, STAKE_TOKEN, 2, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client_address, STAKE_TOKEN, 3, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client_address, STAKE_TOKEN, 4, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client_address, STAKE_TOKEN, 5, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client2_address, STAKE_TOKEN, 6, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client2_address, STAKE_TOKEN, 7, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client2_address, STAKE_TOKEN, 8, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client2_address, STAKE_TOKEN, 9, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client2_address, STAKE_TOKEN, 10, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client3_address, STAKE_TOKEN, 11, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client3_address, STAKE_TOKEN, 12, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client3_address, STAKE_TOKEN, 13, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client3_address, STAKE_TOKEN, 14, &nft_balance, &BoxedBytes::empty());
    blockchain_wrapper.set_nft_balance(&client3_address, STAKE_TOKEN, 15, &nft_balance, &BoxedBytes::empty());

    StakingSetup {
        blockchain_wrapper,
        staking_sc_wrapper,
        owner_address,
        client_address,
        client2_address,
        client3_address
    }
}

#[test]
fn init_test() {
    let _ = DebugApi::dummy();
    let mut sc_setup = setup_staking(cnuns_staking::contract_obj);
    check_origin_epoch(&mut sc_setup, 0);
    check_stake_token(&mut sc_setup);
}

#[test]
fn test_invalid_stake_attempts() {
    let _ = DebugApi::dummy();
    let mut setup = setup_staking(cnuns_staking::contract_obj);
    let nft_balance = num_bigint::ToBigUint::to_biguint(&1).unwrap();

    let mut vec = Vec::new();
    let invalid_nft = TxInputESDT {
        token_identifier: Vec::from(INVALID_STAKE_TOKEN),
        nonce: 1,
        value: nft_balance.clone()
    };
    vec.push(invalid_nft);
    // only stake an invalid NFT
    setup.blockchain_wrapper
        .execute_esdt_multi_transfer(&setup.client_address, &setup.staking_sc_wrapper, &vec, |sc| {
            sc.stake();
        })
        .assert_user_error("one or more NFTs is not eligible for staking");

    for nonce in [1,2,3,4,5].iter() {
        let tx_input = TxInputESDT {
            token_identifier: Vec::from(STAKE_TOKEN),
            nonce: *nonce,
            value: nft_balance.clone()
        };
        vec.push(tx_input);
    }

    // stake invalid NFT combined with other NFTs
    setup.blockchain_wrapper
        .execute_esdt_multi_transfer(&setup.client_address, &setup.staking_sc_wrapper, &vec, |sc| {
            sc.stake();
        })
        .assert_user_error("one or more NFTs is not eligible for staking");

    // stake nothing
    setup.blockchain_wrapper
        .execute_esdt_multi_transfer(&setup.client_address, &setup.staking_sc_wrapper, &Vec::new(), |sc| {
            sc.stake();
        })
        .assert_user_error("cannot stake nothing");
}

#[test]
fn test_stake_unstake_attempts() {
    let _ = DebugApi::dummy();
    let mut sc_setup = setup_staking(cnuns_staking::contract_obj);
    let caller1 = sc_setup.client_address.clone();
    stake_nfts(&mut sc_setup, &caller1, &[1]);
    stake_nfts(&mut sc_setup, &caller1, &[2, 3]);
    stake_nfts(&mut sc_setup, &caller1, &[4, 5]);

    let caller2 = sc_setup.client2_address.clone();
    stake_nfts(&mut sc_setup, &caller2, &[6, 7, 8, 9, 10]);

    let caller3 = sc_setup.client3_address.clone();
    stake_nfts(&mut sc_setup, &caller3, &[11, 12]);
    stake_nfts(&mut sc_setup, &caller3, &[13, 14, 15]);

    unstake_nfts(&mut sc_setup, &caller1, &[1, 2, 3, 4, 5]);
    unstake_nfts(&mut sc_setup, &caller2, &[6, 7, 8, 9, 10]);
    unstake_nfts(&mut sc_setup, &caller3, &[11, 12]);
    unstake_nfts(&mut sc_setup, &caller3, &[13, 14, 15]);
}


#[test]
fn test_simple_split_rewards() {
    let _ = DebugApi::dummy();
    let mut sc_setup = setup_staking(cnuns_staking::contract_obj);

    // stake one nft in epoch 0
    let caller1 = sc_setup.client_address.clone();
    stake_nfts(&mut sc_setup, &caller1, &[1]);
    
    // distribute EGLD rewards in epoch 10
    let mut block_epoch = 10u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    deposit_egld_rewards(&mut sc_setup, REWARD_AMOUNT / block_epoch);
    // distribute ESDT rewards in epoch 20
    block_epoch = 20u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    deposit_esdt_rewards(&mut sc_setup, REWARD_AMOUNT / block_epoch);

    claim_rewards_and_check_balance(&mut sc_setup, &caller1, REWARD_AMOUNT, REWARD_AMOUNT);
}

#[test]
fn test_simple_split_rewards_unstake_before_claim() {
    let _ = DebugApi::dummy();
    let mut sc_setup = setup_staking(cnuns_staking::contract_obj);

    // stake one nft in epoch 0
    let caller1 = sc_setup.client_address.clone();
    stake_nfts(&mut sc_setup, &caller1, &[1]);
    
    // distribute EGLD rewards in epoch 10
    let mut block_epoch = 10u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    deposit_egld_rewards(&mut sc_setup, REWARD_AMOUNT / block_epoch);

    // distribute ESDT rewards in epoch 20
    block_epoch = 20u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    deposit_esdt_rewards(&mut sc_setup, REWARD_AMOUNT / block_epoch);
    unstake_nfts(&mut sc_setup, &caller1, &[1]);

    claim_rewards_and_check_balance(&mut sc_setup, &caller1, REWARD_AMOUNT, REWARD_AMOUNT);
}

#[test]
fn test_complex_split_rewards() {
    let _ = DebugApi::dummy();
    let mut sc_setup = setup_staking(cnuns_staking::contract_obj);

    // stake one nft in epoch 0
    let caller1 = sc_setup.client_address.clone();
    let caller2 = sc_setup.client2_address.clone();
    let caller3 = sc_setup.client3_address.clone();
    stake_nfts(&mut sc_setup, &caller1, &[1]); // 1 unit staked
    
    let mut block_epoch = 5u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);
    stake_nfts(&mut sc_setup, &caller2, &[6, 7, 8]); // 3 more units staked in epoch 5

    // distribute EGLD rewards in epoch 10
    block_epoch = 10u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);
    // epoch 10 expected reward per nft:
    // c1: 10 days * 1 units = 10
    // c2: 5 days * 3 units = 15

    let expected_egld_unit_reward = REWARD_AMOUNT / 25;

    check_total_payable_epochs(&mut sc_setup, &caller1, 0, block_epoch, 10);
    check_total_payable_epochs(&mut sc_setup, &caller2, 0, block_epoch, 15);

    deposit_egld_rewards(&mut sc_setup, expected_egld_unit_reward);

    let c1_egld_rewards = expected_egld_unit_reward * 10;
    let c2_egld_rewards = expected_egld_unit_reward * 15;
    let c3_egld_rewards = 0;

    block_epoch = 15u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    // stats before stakes:
    // c1: 15 epochs * 1 units = 15;
    // c2: 10 epochs * 3 units = 30;


    stake_nfts(&mut sc_setup, &caller1, &[2, 3, 4, 5]);
    stake_nfts(&mut sc_setup, &caller2, &[9, 10]);
    stake_nfts(&mut sc_setup, &caller3, &[11, 12, 13, 14, 15]);
    
    // distribute ESDT rewards in epoch 20
    block_epoch = 20u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);
    
    // stats after stakes:
    // c1: 15 epochs * 1 + 5 epochs * 5 = 15 + 25 = 40
    // c2: 10 epochs * 3 + 5 epochs * 5 = 30 + 25 = 55
    // c3: 25 epochs = 25

    check_total_payable_epochs(&mut sc_setup, &caller1, 0, block_epoch, 40);
    check_total_payable_epochs(&mut sc_setup, &caller2, 0, block_epoch, 55);
    check_total_payable_epochs(&mut sc_setup, &caller3, 0, block_epoch, 25);

    //total = 40 + 55 + 25 = 120
    let expected_esdt_reward_per_unit = REWARD_AMOUNT / 120;
    deposit_esdt_rewards(&mut sc_setup, expected_esdt_reward_per_unit);

    let c1_esdt_rewards = expected_esdt_reward_per_unit * 40;
    let c2_esdt_rewards = expected_esdt_reward_per_unit * 55;
    let c3_esdt_rewards = expected_esdt_reward_per_unit * 25;

    claim_rewards_and_check_balance(&mut sc_setup, &caller1, c1_egld_rewards, c1_esdt_rewards);
    claim_rewards_and_check_balance(&mut sc_setup, &caller2, c2_egld_rewards, c2_esdt_rewards);
    claim_rewards_and_check_balance(&mut sc_setup, &caller3, c3_egld_rewards, c3_esdt_rewards);

    // one more round of rewards, 20 days later
    block_epoch = 40u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    // esdt claim day = 20; since day 20, all get the same 5 * epoch count units (5 nfts staked * 20 epochs = 100 units)
    check_total_payable_epochs(&mut sc_setup, &caller1, 20, block_epoch, 100);
    check_total_payable_epochs(&mut sc_setup, &caller2, 20, block_epoch, 100);
    check_total_payable_epochs(&mut sc_setup, &caller3, 20, block_epoch, 100);

    // egld claim day = 10; went full stake in day 15
    // this means:
    // c1 = 1 nft * (15 - 10) + 5 nft * (40 - 15)
    // c2 = 3 * 5 + 25 * 5
    // c3 = 60-15 * 5 = 45 * 5;
    // total = 5 + 125 + 15 + 125 + 100 = 370

    check_total_payable_epochs(&mut sc_setup, &caller1, 10, block_epoch, 130);
    check_total_payable_epochs(&mut sc_setup, &caller2, 10, block_epoch, 140);
    check_total_payable_epochs(&mut sc_setup, &caller3, 10, block_epoch, 125);
}

#[test]
fn test_end_stake() {
    let _ = DebugApi::dummy();
    let mut sc_setup = setup_staking(cnuns_staking::contract_obj);

    let caller1 = sc_setup.client_address.clone();
    stake_nfts(&mut sc_setup, &caller1, &[1]);
    
    // distribute EGLD rewards in epoch 10
    let mut block_epoch = 10u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    deposit_egld_rewards(&mut sc_setup, REWARD_AMOUNT / block_epoch);

    // distribute ESDT rewards in epoch 20
    block_epoch = 20u64;
    sc_setup.blockchain_wrapper.set_block_epoch(block_epoch);

    deposit_esdt_rewards(&mut sc_setup, REWARD_AMOUNT / block_epoch);
    trigger_end_stake(&mut sc_setup);

    sc_setup.blockchain_wrapper
            .check_nft_balance::<ManagedBuffer<DebugApi>>(&caller1, &STAKE_TOKEN, 1, &rust_biguint!(1), Option::None);

    sc_setup.blockchain_wrapper
        .check_esdt_balance(&caller1, REWARD_TOKEN, &rust_biguint!(REWARD_AMOUNT));
    sc_setup.blockchain_wrapper
        .check_egld_balance(&caller1, &rust_biguint!(REWARD_AMOUNT));

}

/* Helper functions */
fn check_origin_epoch<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
    expected: u64,
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    setup.blockchain_wrapper
        .execute_query(&setup.staking_sc_wrapper, |sc| {
            let origin_epoch = sc.get_origin_epoch();
            assert_eq!(expected, origin_epoch);
        })
        .assert_ok();
}

fn check_stake_token<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    setup.blockchain_wrapper
        .execute_query(&setup.staking_sc_wrapper, |sc| {
            let stake_token = sc.get_stake_token();
            let expected = managed_token_id!(STAKE_TOKEN);
            assert_eq!(expected, stake_token);
        })
        .assert_ok();
}

fn stake_nfts<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
    caller: &Address,
    nonces: &[u64],
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    let nft_balance = num_bigint::ToBigUint::to_biguint(&1).unwrap();

    let mut vec = Vec::new();
    for nonce in nonces.iter() {
        let tx_input = TxInputESDT {
            token_identifier: Vec::from(STAKE_TOKEN),
            nonce: *nonce,
            value: nft_balance.clone()
        };
        vec.push(tx_input);
    }
    
    setup.blockchain_wrapper
        .execute_esdt_multi_transfer(caller, &setup.staking_sc_wrapper, &vec, |sc| {
            sc.stake();
        })
        .assert_ok();
}

fn unstake_nfts<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
    caller: &Address,
    nonces: &[u64],
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    let nft_balance = num_bigint::ToBigUint::to_biguint(&1).unwrap();
    let zero = num_bigint::ToBigUint::to_biguint(&0).unwrap();
    let mut unstake_params = ManagedVec::new();
    // let mut vec = Vec::new();
    for nonce in nonces.iter() {
        unstake_params.push(*nonce);
    }
    
    setup.blockchain_wrapper
        .execute_tx(caller, &setup.staking_sc_wrapper, &zero, |sc| {
            sc.unstake(unstake_params);
        })
        .assert_ok();

    for nonce in nonces.iter() {
        setup.blockchain_wrapper
            .check_nft_balance::<ManagedBuffer<DebugApi>>(caller, &STAKE_TOKEN, *nonce, &nft_balance, Option::None);
    }
}

fn deposit_egld_rewards<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
    expected_reward_per_unit: u64,
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    let reward_amount = num_bigint::ToBigUint::to_biguint(&REWARD_AMOUNT).unwrap();
    
    setup.blockchain_wrapper
        .execute_tx(&setup.owner_address, &setup.staking_sc_wrapper, &reward_amount, |sc| {
            let reward_per_epoch_per_nonce = sc.deposit_reward();
            assert_eq!(managed_biguint!(expected_reward_per_unit), reward_per_epoch_per_nonce);
        })
        .assert_ok();
}

fn deposit_esdt_rewards<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
    expected_reward_per_unit: u64,
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    let reward_amount = num_bigint::ToBigUint::to_biguint(&REWARD_AMOUNT).unwrap();
    
    setup.blockchain_wrapper
        .execute_esdt_transfer(&setup.owner_address, &setup.staking_sc_wrapper, &REWARD_TOKEN, 0u64, &reward_amount, |sc| {
            let reward_per_epoch_per_nonce = sc.deposit_reward();
            assert_eq!(managed_biguint!(expected_reward_per_unit), reward_per_epoch_per_nonce);
        })
        .assert_ok();
}

fn trigger_end_stake<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    let reward_amount = num_bigint::ToBigUint::to_biguint(&REWARD_AMOUNT).unwrap();
    
    setup.blockchain_wrapper
        .execute_tx(&setup.owner_address, &setup.staking_sc_wrapper, &rust_biguint!(0), |sc| {
            sc.end_staking();
        })
        .assert_ok();
}

fn claim_rewards_and_check_balance<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
    caller: &Address,
    expected_total_egld_reward: u64,
    expected_total_esdt_reward: u64,
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    let zero = num_bigint::ToBigUint::to_biguint(&0).unwrap();

    setup.blockchain_wrapper
        .execute_tx(caller, &setup.staking_sc_wrapper, &zero, |sc| {
            sc.claim_reward();
        })
        .assert_ok();

    let egld_balance = setup.blockchain_wrapper
        .get_egld_balance(caller);
    let esdt_balance = setup.blockchain_wrapper
        .get_esdt_balance(caller, &REWARD_TOKEN, 0u64);

    let expected_egld_balance = num_bigint::ToBigUint::to_biguint(&expected_total_egld_reward).unwrap();
    let expected_esdt_balance = num_bigint::ToBigUint::to_biguint(&expected_total_esdt_reward).unwrap();

    assert_eq!(expected_egld_balance, egld_balance);
    assert_eq!(expected_esdt_balance, esdt_balance);
}

fn check_total_payable_epochs<StakingObjBuilder>(
    setup: &mut StakingSetup<StakingObjBuilder>,
    address: &Address,
    last_payment_epoch: u64,
    current_epoch: u64,
    expected: u64
) where
    StakingObjBuilder: 'static + Copy + Fn() -> cnuns_staking::ContractObj<DebugApi>,
{
    setup.blockchain_wrapper
        .execute_query(&setup.staking_sc_wrapper, |sc| {
            let total_payable_epochs = sc.get_total_payable_epochs(current_epoch, last_payment_epoch, &managed_address!(address));
            assert_eq!(expected, total_payable_epochs);
        })
        .assert_ok();
}