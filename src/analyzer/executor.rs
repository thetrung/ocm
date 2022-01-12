
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

fn correct_lots_qty(final_ring: &Vec<String>, prices: &Vec<f64>, 
    optimal_invest: f64, quantity_info: &HashMap<String, QuantityInfo>) -> f64 {
    //
    // Buy > Sell > Sell
    //
    let decimal_place = quantity_info[&final_ring[0]].stepSizeDecimal as f64;

    let one_decimal:f64 = 10.0;
    let move_decimal = one_decimal.powf(decimal_place);
    let fees = 1.0 - (0.075 / 100.0);
    let buy_qty = optimal_invest/prices[0] * fees;
    let qty_first_buy = f64::trunc(buy_qty  * move_decimal) / move_decimal;
    
    println!("> qty: {} => {} as {} has only {} decimals.", 
    buy_qty, qty_first_buy, quantity_info[&final_ring[0]].stepSize ,decimal_place);
    return qty_first_buy;
}

/// Execute best ring found in previous round result.
pub fn execute_final_ring(account: &Account, ring_component: &RingComponent,
    final_ring: &Vec<String>, prices: &Vec<f64>, config_invest: f64, 
    quantity_info: &HashMap<String, QuantityInfo>) -> Option<f64> {
    

    let mut order_result = false;
    let _current_balance = get_balance(&account, &ring_component.stablecoin).unwrap();
    let optimal_invest = if config_invest > _current_balance { _current_balance } else { config_invest };
    let mut balance_qty:f64 = correct_lots_qty(final_ring, prices, optimal_invest, quantity_info);
    // for testing purpose.
    return None;
    //
    // 1. Buy OOKI-BUSD
    //
    println!("> limit_buy: {} {} at {}", &balance_qty.to_string().green(), &final_ring[0].green(), &prices[0].to_string().yellow());
    match account.limit_buy(&final_ring[0], balance_qty, prices[0]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, &final_ring[0]),
        Err(e) => println!("Error: {:?}", e),
    }
    if order_result { 
        balance_qty = get_balance(&account, &ring_component.symbol).unwrap();
        println!("> executed: limit_buy for {:?} {}", balance_qty, &final_ring[0]);
    }
    else { return None }
    
    //
    // 2. Sell OOKI-BNB
    //
    println!("> limit_sell: {} {} at {}", &balance_qty.to_string().green(), &final_ring[1].green(), &prices[1].to_string().yellow());
    match account.limit_sell(&final_ring[1], balance_qty, prices[1]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, &final_ring[1]),
        Err(e) => println!("Error: {:?}", e),
    }
    if order_result { 
        balance_qty = get_balance(&account, &ring_component.bridge).unwrap();
        println!("> executed: limit_sell for {:?} {}", balance_qty, &final_ring[1]);
    }
    else { return None }

    //
    // 3. Sell BNB-BUSD
    //
    println!("> limit_sell: {} {} at {}", &balance_qty.to_string().green(), &final_ring[2].green(), &prices[2].to_string().yellow());
    match account.limit_sell(&final_ring[2], balance_qty, prices[2]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, &final_ring[2]),
        Err(e) => println!("Error: {:?}", e),
    }
    if order_result { 
        balance_qty = get_balance(&account, &ring_component.stablecoin).unwrap();
        println!("> executed: limit_sell for {:?} {}", balance_qty, &final_ring[2]); 
    }
    else { return None }

    return Some(balance_qty);
}