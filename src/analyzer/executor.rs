
use colored::*;
use configparser::ini::Ini;

use binance::api::*;
use binance::account::*;
use binance::model::Transaction;
use binance::errors::ErrorKind as BinanceLibErrorKind;

use std::{
    collections::HashMap,
    thread::{self, JoinHandle} ,
    time::{Duration, SystemTime}};

use crate::exchangeinfo::QuantityInfo;
use crate::analyzer::RingComponent;

const FEES: f64 = 0.99925; // no BNB = 0.999
/// Wait time between orders
const POLLING_ORDER: Duration = Duration::from_millis(500);
// const POLLING_ORDER_WAIT: Duration = Duration::from_millis(1000);
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
        match account.order_status(symbol, order_id) {
            Ok(answer) => {
                match answer.status.as_str() {
                    "FILLED" => {
                        println!("> executed qty: {}/{}", answer.executed_qty.green(), qty);
                        return true;
                    },  // can move on next symbol
                    "CANCELED" => return false, // on purpose ;) move to next round ?
                    _ => {}//println!("> {} {} is {:?}", qty, &symbol ,answer.status)
                }
            },
            Err(e) => println!("Error: {:?}", e),
        }
        thread::sleep(POLLING_ORDER);
    }            
}

/// Get balance of any symbol in account.
pub fn get_balance(account: &Account, symbol: &str) -> Option<f64>{
    match account.get_balance(symbol) {
        Ok(answer) => {
            let qty = answer.free.parse::<f64>().unwrap();
            println!("> balance: {} {}", qty, symbol);
            return Some(qty);
        },
        Err(e) => { println!("{:?}", e); return None; }
    }
}

fn correct_lots_qty(symbol: &str, qty: f64, quantity_info: &HashMap<String, QuantityInfo>) -> f64 {
    let move_qty = quantity_info[symbol].move_qty;
    let corrected_qty = f64::trunc(qty  * move_qty) / move_qty;
    return corrected_qty;
}

fn format_error(e: BinanceLibErrorKind){
    match e {
        BinanceLibErrorKind::BinanceError(response) => println!("> error: {}", response.msg),
        _ => {}
    }
}

/// Execute best ring found in previous round result.
pub fn execute_final_ring(account: &Account, benchmark: &SystemTime, ring_component: &RingComponent,
    final_ring: &Vec<String>, prices: &Vec<[f64;2]>, config_invest: f64, 
    quantity_info: &HashMap<String, QuantityInfo>) -> Option<f64> {
    
    //> for testing purpose.
    // return None;

    // states
    let mut order_result = false;

    // correct lots + step_size
    let mut symbol:&str;
    let mut step_qty:f64 = 0.0;
    let mut step_price:f64 = 0.0;
    let mut balance_qty:f64 = 0.0;
    
    // prepare balance 
    let _current_balance = get_balance(&account, &ring_component.stablecoin).unwrap();
    let optimal_invest = if config_invest > _current_balance { _current_balance } else { config_invest };

    //
    // 1. Buy OOKI-BUSD
    //
    symbol = &final_ring[0];
    // step_qty = quantity_info[symbol].step_qty;
    // step_price = quantity_info[symbol].step_price;
    let custom_first_buy = optimal_invest/(prices[0][0] + quantity_info[symbol].step_price) * FEES;
    balance_qty = correct_lots_qty(symbol, custom_first_buy, quantity_info);
    println!("> limit_buy: {} {} at {}", &balance_qty.to_string().green(), symbol.green(), &prices[0][0].to_string().yellow());
    match account.market_buy(symbol, balance_qty) {
    // match account.limit_buy(symbol, balance_qty, prices[0][0]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => format_error(e.0),
    }
    if order_result {
        let tickers_update_time = benchmark.elapsed().unwrap().as_millis().to_string();
        balance_qty = get_balance(&account, &ring_component.symbol).unwrap();
        println!("{}", format!(
            "> executed: limit_buy for {:?} {} in {} ms.", balance_qty, symbol, tickers_update_time).green());
    }
    else { return None }
    //
    // 2. Sell OOKI-BTC
    //
    symbol = &final_ring[1];
    step_price = quantity_info[symbol].step_price;
    balance_qty = correct_lots_qty(symbol, balance_qty, quantity_info); println!("> corrected balance_qty: {}", balance_qty);
    println!("> limit_sell: {} {} at {} + {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[1][0].to_string().yellow(), &step_price);
    match account.limit_sell(symbol, balance_qty, prices[1][0]+step_price) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => format_error(e.0),
    }
    if order_result {
        let tickers_update_time = benchmark.elapsed().unwrap().as_millis().to_string();
        balance_qty = get_balance(&account, &ring_component.bridge).unwrap();
        println!("{}", format!("> executed: limit_sell for for {:?} {} in {} ms.", balance_qty, symbol, tickers_update_time).green());
    }
    else { return None }

    //
    // 3. Sell BTC-BUSD
    //
    symbol = &final_ring[2];
    step_price = quantity_info[symbol].step_price;
    balance_qty = correct_lots_qty(symbol, balance_qty, quantity_info);
    println!("> limit_sell: {} {} at {} + {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[2][0].to_string().yellow(), step_price);
    match account.limit_sell(symbol, balance_qty, prices[2][0] + 0.25 * 1000.0 * step_price) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => format_error(e.0),
    }
    if order_result { 
        let tickers_update_time = benchmark.elapsed().unwrap().as_millis().to_string();
        balance_qty = get_balance(&account, &ring_component.stablecoin).unwrap();
        println!("{}", format!("> executed: limit_sell for for {:?} {} in {} ms.", balance_qty, symbol, tickers_update_time).green());
    }
    else { return None }

    return Some(balance_qty);
}