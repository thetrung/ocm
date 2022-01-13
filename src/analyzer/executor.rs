
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

/// Poll and Wait until an order is filled.
fn polling_order(account: &Account, order_id: u64, qty: f64, symbol: &str) -> Option<f64> {
    println!("> order: #{} for {} {}", &order_id.to_string().yellow(), qty.to_string().green(), &symbol.green());
    loop {
        match account.order_status(symbol, order_id) {
            Ok(answer) => {
                match answer.status.as_str() {
                    "FILLED" => {
                        println!("> executed qty: {}/{}", answer.executed_qty.green(), qty);
                        return Some(answer.executed_qty.parse::<f64>().unwrap());
                    },  // can move on next symbol
                    "CANCELED" => return None, // on purpose ;) move to next round ?
                    _ => {}//println!("> {} {} is {:?}", qty, &symbol ,answer.status)
                }
            },
            Err(e) => format_error(e.0),
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
fn correct_price_filter(symbol: &str, quantity_info: &HashMap<String, QuantityInfo>,  price: f64) -> f64 {
    let move_price = quantity_info[symbol].move_price;
    return f64::trunc(price  * move_price) / move_price;
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

fn format_result(balance_qty:f64, symbol: &str, benchmark: &SystemTime){
    println!("{}", format!(
        "> result: {:?} {} after {} ms.\n", 
        balance_qty, 
        symbol, 
        benchmark.elapsed().unwrap().as_millis().to_string()).green());
}

/// Execute best ring found in previous round result.
pub fn execute_final_ring(account: &Account, ring_component: &RingComponent,
    final_ring: &Vec<String>, prices: &Vec<[f64;2]>, config_invest: f64, 
    quantity_info: &HashMap<String, QuantityInfo>) -> Option<f64> {
    
    let benchmark = SystemTime::now();
    println!("> -------------------------------------------------- <");
    //> for testing purpose.
    // return Some(0.0);

    // states
    let mut order_result:Option<f64> = None;

    // correct lots + step_size
    let mut symbol:&str;
    let mut step_qty:f64 = 0.0;
    let mut step_price:f64 = 0.0;
    let mut balance_qty:f64 = 0.0;
    let mut custom_price:f64 = 0.0;
    
    // prepare balance 
    let _current_balance = get_balance(&account, &ring_component.stablecoin).unwrap(); println!();
    let optimal_invest = if config_invest > _current_balance { _current_balance } else { config_invest };


    // step_qty = quantity_info[symbol].step_qty;
    // step_price = quantity_info[symbol].step_price;
    //
    // 1. Buy OOKI-BUSD
    //
    symbol = &final_ring[0];
    let first_order = optimal_invest/(prices[0][0] + 1.0 * quantity_info[symbol].step_price) * FEES;

    balance_qty = correct_lots_qty(symbol, first_order, quantity_info);
    println!("> limit_buy: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[0][0].to_string().yellow());
    // match account.limit_buy(symbol, balance_qty, prices[0][0]) {
    match account.market_buy(symbol, balance_qty) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => { 
            format_error(e.0); 
            // return None; 
        }
    }
    match order_result {
        Some(executed_qty) => {
            balance_qty = executed_qty;
            format_result(balance_qty, &ring_component.symbol, &benchmark);
        }
        None => return Some(-1.0) 
    }
    //
    // 2. Sell OOKI-BTC
    //
    symbol = &final_ring[1];
    step_price = quantity_info[symbol].step_price;
    balance_qty = correct_lots_qty(symbol, balance_qty, quantity_info); 
    custom_price = correct_price_filter(symbol, quantity_info, prices[1][0] - 1.0 * step_price);
    println!("> limit_sell: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[1][0].to_string().yellow());
    match account.limit_sell(symbol, balance_qty, custom_price) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => { 
            format_error(e.0);
            return None
        },
    }
    match order_result {
        Some(_) => { // Have to refresh because it's no longer executed qty.
            balance_qty = get_balance(&account, &ring_component.bridge).unwrap(); 
            format_result(balance_qty, &ring_component.bridge, &benchmark);
        }
        None => return None 
    }

    //
    // 3. Sell BTC-BUSD
    //
    symbol = &final_ring[2];
    balance_qty = correct_lots_qty(symbol, balance_qty, quantity_info);
    println!("> market_sell: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[2][0].to_string().yellow());
    // match account.limit_sell(symbol, balance_qty, prices[2][0]) {
    match account.market_sell(symbol, balance_qty) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, symbol),
        Err(e) => { 
            format_error(e.0); 
            return None; // Error
        }
    }
    match order_result {
        Some(_) => {
            balance_qty = get_balance(&account, &ring_component.stablecoin).unwrap();
            format_result(balance_qty, &ring_component.stablecoin, &benchmark);
        }
        None => return None 
    }

    return Some(balance_qty);
}