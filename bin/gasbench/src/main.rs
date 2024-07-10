use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use alloy::primitives::{Address, BlockNumber, U256};
use alloy::providers::WsConnect;
use alloy_network::Network;
use alloy_primitives::Bytes;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockNumberOrTag, TransactionInput, TransactionRequest};
use alloy_transport::Transport;
use clap::Parser;
use colored::*;
use eyre::{eyre, OptionExt, Result};
use lazy_static::lazy_static;
use log::{debug, error, info};
use revm::db::EmptyDB;
use revm::InMemoryDB;
use revm::primitives::Env;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use debug_provider::{AnvilDebugProviderFactory, AnvilProviderExt, DebugProviderExt};
use defi_abi::IERC20::IERC20Instance;
use defi_actors::{fetch_and_add_pool_by_address, preload_market_state};
use defi_entities::{Market, MarketState, NWETH, PoolClass, PoolWrapper, Swap, SwapAmountType, SwapLine, SwapPath, Token};
use loom_actors::SharedState;
use loom_multicaller::{MulticallerDeployer, MulticallerSwapEncoder, SwapEncoder};
use loom_revm_db::LoomInMemoryDB;
use loom_utils::evm::evm_call;
use loom_utils::remv_db_direct_access::calc_hashmap_cell;

use crate::balances::set_balance;
use crate::cli::Cli;
use crate::dto::SwapLineDTO;
use crate::preloader::{preload_pools, WETH_ADDRESS};
use crate::soltest::create_sol_test;

mod cli;
mod dto;
mod soltest;
mod preloader;
mod balances;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SwapPathDTO {
    tokens: Vec<Address>,
    pools: Vec<Address>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::try_parse()?;

    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or("debug,alloy_rpc_client=off"),
    );
    let block_number = 20089277u64;

    println!("Hello, block {block_number}!");
    let client = AnvilDebugProviderFactory::from_node_on_block(
        "ws://falcon.loop:8008/looper".to_string(),
        BlockNumber::from(block_number),
    ).await?;
    //let client = ProviderBuilder::new().on_ws(WsConnect::new("ws://localhost:8545")).await?.boxed();

    //preset_balances(client.clone()).await?;

    let block_header = client
        .get_block_by_number(BlockNumberOrTag::Number(block_number), false)
        .await?
        .unwrap()
        .header;

    let operator_address = Address::repeat_byte(0x12);
    let multicaller_address = Address::repeat_byte(0x78);

    // Set Multicaller code
    let multicaller_address = MulticallerDeployer::new()
        .set_code(client.clone(), multicaller_address)
        .await?
        .address()
        .ok_or_eyre("MULTICALLER_NOT_DEPLOYED")?;
    info!("Multicaller deployed at {:?}", multicaller_address);

    // SET Multicaller WETH balance
    set_balance(client.clone(), multicaller_address, *WETH_ADDRESS).await?;


    // Initialization
    let cache_db = LoomInMemoryDB::default();

    let market_instance = Market::default();

    let market_state_instance = MarketState::new(cache_db.clone());

    let market_instance = SharedState::new(market_instance);

    let market_state_instance = SharedState::new(market_state_instance);

    let encoder = Arc::new(MulticallerSwapEncoder::new(multicaller_address));

    //preload state
    preload_market_state(
        client.clone(),
        vec![multicaller_address],
        market_state_instance.clone(),
    )
        .await?;

    //Preloading market
    preload_pools(
        client.clone(),
        market_instance.clone(),
        market_state_instance.clone(),
    )
        .await?;

    let market = market_instance.read().await;

    // Getting swap directions
    let pool_address: Address = "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".parse()?;

    let pool = market
        .get_pool(&pool_address)
        .ok_or_eyre("POOL_NOT_FOUND")?;

    let swap_directions = pool.get_swap_directions();

    let mut btree_map: BTreeMap<PoolWrapper, Vec<(Address, Address)>> = BTreeMap::new();

    btree_map.insert(pool.clone(), swap_directions);

    let swap_paths = market.build_swap_path_vec(&btree_map)?;


    let db = market_state_instance.read().await.state_db.clone();

    let mut env = Env::default();

    env.block.number = U256::from(block_header.number.unwrap_or_default());
    env.block.timestamp = U256::from(block_header.timestamp);
    //env.block.basefee = U256::from(block_header.base_fee_per_gas.unwrap_or_default());

    let in_amount_f64 = 1.0;
    let in_amount = NWETH::from_float(in_amount_f64);

    let mut gas_used_map: HashMap<SwapLineDTO, u64> = HashMap::new();
    let mut calldata_map: HashMap<SwapLineDTO, Bytes> = HashMap::new();

    // Make tests


    for (i, s) in swap_paths.iter().enumerate() {
        if !s.tokens[0].is_weth() {
            continue;
        }
        let sp = s.as_ref().clone();
        let sp_dto: SwapLineDTO = (&sp).into();

        let mut swapline = SwapLine {
            path: sp,
            amount_in: SwapAmountType::Set(in_amount),
            ..SwapLine::default()
        };

        match swapline.calculate_with_in_amount(&db, env.clone(), in_amount) {
            Ok((out_amount, gas_used)) => {
                info!(
                    "{} gas: {}  amount {} -> {}",
                    sp_dto,
                    gas_used,
                    in_amount_f64,
                    NWETH::to_float(out_amount)
                );
                swapline.amount_out = SwapAmountType::Set(out_amount)
            }
            Err(e) => {
                error!("calculate_with_in_amount error : {:?}", e);
            }
        }
        let swap = Swap::BackrunSwapLine(swapline);

        let calls = encoder.make_calls(&swap)?;
        let (to, payload) = encoder.encode_calls(calls)?;

        calldata_map.insert(s.clone().as_ref().into(), payload.clone());

        let tx_request = TransactionRequest::default()
            .to(to)
            .from(operator_address)
            .input(TransactionInput::new(payload));

        let gas_used = match client.estimate_gas(&tx_request).await {
            Ok(gas_needed) => {
                //info!("Gas required:  {gas_needed}");
                gas_needed as u64
            }
            Err(e) => {
                error!("Gas estimation error : {e}");
                0
            }
        };

        gas_used_map.insert(s.clone().as_ref().into(), gas_used);
    }


    if let Some(bench_file) = cli.file {
        if cli.anvil { // Save anvil test data
            let mut calldata_vec: Vec<(SwapLineDTO, Bytes)> = calldata_map.into_iter().map(|(k, v)| (k, v)).collect();
            calldata_vec.sort_by(|a, b| a.0.cmp(&b.0));
            let calldata_vec: Vec<(String, Bytes)> = calldata_vec.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
            let test_data = create_sol_test(calldata_vec);
            println!("{}", test_data);
            let mut file = File::create(bench_file).await?;
            file.write_all(test_data.as_bytes()).await?;
        } else if cli.save { // Save benchmark results
            let results: Vec<(SwapLineDTO, u64)> = gas_used_map.into_iter().collect();

            let json_string = serde_json::to_string_pretty(&results)?;

            let mut file = File::create(bench_file).await?;
            file.write_all(json_string.as_bytes()).await?;
        } else { // Compare benchmark results
            let mut file = File::open(bench_file).await?;
            let mut json_string = String::new();
            file.read_to_string(&mut json_string).await?;

            let stored_results: Vec<(SwapLineDTO, u64)> = serde_json::from_str(&json_string)?;

            let stored_gas_map: HashMap<SwapLineDTO, u64> = stored_results.clone().into_iter().map(|(k, v)| (k, v)).collect();

            for (current_entry, gas) in gas_used_map.iter() {
                match stored_gas_map.get(current_entry) {
                    Some(stored_gas) => {
                        let change_i: i64 = *gas as i64 - *stored_gas as i64;
                        let change = format!("{change_i}");
                        let change = if change_i > 0 {
                            change.red()
                        } else if change_i < 0 {
                            change.green()
                        } else {
                            change.normal()
                        };
                        println!("{} : {} {} - {} ", change, current_entry, gas, stored_gas, );
                    }
                    None => {
                        println!("{} : {} {}", "NO_DATA".green(), current_entry, gas, );
                    }
                }
            }
        }
    }


    Ok(())
}
