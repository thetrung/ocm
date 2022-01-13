
use colored::*;
use configparser::ini::Ini;

use binance::api::*;
use binance::account::*;
use binance::model::Transaction;

use std::{
    collections::HashMap,
    thread::{self, JoinHandle} ,
    time::{Duration, SystemTime}};

use crate::exchangeinfo::QuantityInfo;
use crate::analyzer::RingComponent;

/// Wait time between orders
const POLLING_ORDER: Duration = Duration::from_millis(1000);
const POLLING_ORDER_WAIT: Duration = Duration::from_millis(1000);
// NOTE :
// We may setup a wait time so the bot auto cancel order if waited too long,
// then it can auto-market sell the asset and reboot the loop, but this come at cost of fund loss.
//
// 1. We may need to refresh tickers during polling, 
// in case, price gap is too big, we need to cancel order + market sell. 
//

/// Poll and Wait until an order is filled.
fn polling_order(account: &Account, order_id: u64, qty: f64, symbol: &str) -> bool {
    println!("> order: #{} for {} {}", &order_id.to_string().yellow(), qty.to_string().green(), &symbol.green());
    loop {
        thread::sleep(POLLING_ORDER);

        match account.order_status(symbol, order_id) {
            Ok(answer) => {
                match answer.status.as_str() {
                    "FILLED" => {
                        println!("> executed qty: {}/{}\n", answer.executed_qty.green(), qty);
                        return true;
                    },  // can move on next symbol
                    "CANCELED" => return false, // on purpose ;) move to next round ?
                    _ => {}//println!("> {} {} is {:?}", qty, &symbol ,answer.status)
                }
            },
            Err(e) => println!("Error: {:?}", e),
        }
    }            
}

/// Get balance of any symbol in account.
fn get_balance(account: &Account, symbol: &str) -> Option<f64>{
    match account.get_balance(symbol) {
        Ok(answer) => {
            let qty = answer.free.parse::<f64>().unwrap();
            println!("> balance: {} {}", qty, symbol);
            return Some(qty);
        },
        Err(e) => { println!("{:?}", e); return None; }
    }
}

fn correct_lots_qty(ring: &str, qty: f64, quantity_info: &HashMap<String, QuantityInfo>) -> f64 {
    //
    // Buy > Sell > Sell
    //
    let fees = 1.0 - (0.075 / 100.0);
    let buy_qty = qty * fees;
    let move_qty = quantity_info[ring].move_qty;
    let corrected_qty = f64::trunc(buy_qty  * move_qty) / move_qty;
    
    println!("> qty: {} => {} as {} has only {} decimals.", 
    buy_qty, corrected_qty, quantity_info[ring].step_qty , move_qty);
    return corrected_qty;
}

/// Execute best ring found in previous round result.
pub fn execute_final_ring(account: &Account, benchmark: &SystemTime, ring_component: &RingComponent,
    final_ring: &Vec<String>, prices: &Vec<[f64;2]>, config_invest: f64, 
    quantity_info: &HashMap<String, QuantityInfo>) -> Option<f64> {
    
    //> for testing purpose.
    return None;

    // states
    let mut order_result = false;

    // correct lots + step_size
    let mut symbol:&str;
    let mut step_qty:f64 = 0.0;
    let mut balance_qty:f64 = 0.0;
    
    // prepare balance 
    let _current_balance = get_balance(&account, &ring_component.stablecoin).unwrap();
    let optimal_invest = if config_invest > _current_balance { _current_balance } else { config_invest };

    //
    // 1. Buy OOKI-BUSD
    //
    symbol = &final_ring[0];
    step_qty = quantity_info[symbol].step_qty;
    balance_qty = correct_lots_qty(symbol, optimal_invest/prices[0][0], quantity_info);
    println!("> limit_buy: {} {} at {}", &balance_qty.to_string().green(), symbol.green(), &prices[0][0].to_string().yellow());
    match account.market_buy(symbol, balance_qty) {
    // match account.limit_buy(symbol, balance_qty, prices[0][0]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => println!("Error: {:?}", e),
    }
    if order_result {
        let mut tickers_update_time:String = String::new();
        match benchmark.elapsed() {
            Ok(elapsed) => tickers_update_time = elapsed.as_millis().to_string(),
            Err(e) => println!("> can't benchmark update_orderbooks: {:?}", e)
        }
        balance_qty = get_balance(&account, &ring_component.symbol).unwrap();
        println!("> executed: limit_buy for {:?} {} in {:?}", balance_qty, symbol, tickers_update_time);
    }
    else { return None }
    //
    // 2. Sell OOKI-BNB
    //
    symbol = &final_ring[1];
    step_qty = quantity_info[symbol].step_qty;
    balance_qty = correct_lots_qty(symbol, balance_qty, quantity_info);
    println!("> limit_sell: {} {} at {}", &balance_qty.to_string().green(), symbol.green(), &prices[1][0].to_string().yellow());
    match account.limit_sell(symbol, balance_qty, prices[1][0]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => println!("Error: {:?}", e),
    }
    if order_result { 
        let mut tickers_update_time:String = String::new();
        match benchmark.elapsed() {
            Ok(elapsed) => tickers_update_time = elapsed.as_millis().to_string(),
            Err(e) => println!("> can't benchmark update_orderbooks: {:?}", e)
        }
        balance_qty = get_balance(&account, &ring_component.bridge).unwrap();
        println!("> executed: limit_sell for for {:?} {} in {:?}", balance_qty, symbol, tickers_update_time);
    }
    else { return None }

    //
    // 3. Sell BNB-BUSD
    //
    symbol = &final_ring[2];
    step_qty = quantity_info[symbol].step_qty;
    balance_qty = correct_lots_qty(symbol, balance_qty, quantity_info);
    println!("> limit_sell: {} {} at {}", &balance_qty.to_string().green(), symbol.green(), &prices[2][0].to_string().yellow());
    match account.limit_sell(symbol, balance_qty, prices[2][0]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => println!("Error: {:?}", e),
    }
    if order_result { 
        let mut tickers_update_time:String = String::new();
        match benchmark.elapsed() {
            Ok(elapsed) => tickers_update_time = elapsed.as_millis().to_string(),
            Err(e) => println!("> can't benchmark update_orderbooks: {:?}", e)
        }
        balance_qty = get_balance(&account, &ring_component.stablecoin).unwrap();
        println!("> executed: limit_sell for for {:?} {} in {:?}", balance_qty, symbol, tickers_update_time);
    }
    else { return None }

    return Some(balance_qty);
}