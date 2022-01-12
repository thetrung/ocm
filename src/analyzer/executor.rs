use binance::api::*;
use binance::account::*;
use binance::model::Transaction;
use configparser::ini::Ini;

use std::{
    collections::HashMap,
    thread::{self, JoinHandle} ,
    time::{Duration, SystemTime}};

use crate::exchangeinfo::QuantityInfo;
use crate::analyzer::RingComponent;

/// Wait time between orders
const POLLING_ORDER: Duration = Duration::from_millis(1000);

/// Poll and Wait until an order is filled.
fn polling_order(account: &Account, order_id: u64, qty: f64, symbol: &str) -> bool {
    println!("> new order_id {:?} for {} {}", &order_id, qty, &symbol);
    loop {
        thread::sleep(POLLING_ORDER);

        match account.order_status(symbol, order_id) {
            Ok(answer) => {
                match answer.status.as_str() {
                    "FILLED" => {
                        println!("> executed qty: {}", answer.executed_qty);
                        return true;
                    },  // can move on next symbol
                    "CANCELED" => return false, // on purpose ;) move to next round ?
                    _ => print!(".")//println!("> {} {} is {:?}", qty, &symbol ,answer.status)
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
            println!("> Balance: {} {}", qty, symbol);
            return Some(qty);
        },
        Err(e) => { println!("{:?}", e); return None; }
    }
}

/// Execute best ring found in previous round result.
pub fn execute_final_ring(account: &Account, ring_component: &RingComponent,
    final_ring: &Vec<String>, prices: &Vec<f64>, 
    optimal_invest: f64, 
    quantity_info: &HashMap<String, QuantityInfo>){
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
    // return;

    //
    // Note: 
    // Ok, situation here is, once we submit a buy order, 
    // we need to wait until the order is filled before calling the next one.
    // 
    // 1. Execute Order
    // 2. Polling orderStatus until it's filled
    // 3. Fetch filled symbol Qty (eQty)
    // 4. Execute next order by that
    // 5. Repeat the whole ring
    //
    let mut balance_qty:f64 = 0.0;
    let mut order_result = false;
    //
    // 1. Buy OOKI-BUSD
    //
    println!(">LIMIT_BUY {} {} {}", &final_ring[0], &qty_first_buy, &prices[0]);
    match account.limit_buy(&final_ring[0], qty_first_buy, prices[0]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, qty_first_buy, &final_ring[0]),
        Err(e) => println!("Error: {:?}", e),
    }
    balance_qty = get_balance(&account, &ring_component.symbol).unwrap();
    if order_result { println!("executed LIMIT_BUY {:?} {}", balance_qty, &final_ring[0]) }
    else { return }
    
    //
    // 2. Sell OOKI-BNB
    //
    println!(">LIMIT_SELL {} {} {}", &final_ring[1], &balance_qty, &prices[1]);
    match account.limit_sell(&final_ring[1], balance_qty, prices[1]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, &final_ring[1]),
        Err(e) => println!("Error: {:?}", e),
    }
    balance_qty = get_balance(&account, &ring_component.bridge).unwrap();
    if order_result { println!("executed LIMIT_SELL {:?} {}", balance_qty, &final_ring[1]) }
    else { return }

    //
    // 3. Sell BNB-BUSD
    //
    println!(">LIMIT_SELL {} {} {}", &final_ring[2], &balance_qty, &prices[2]);
    match account.limit_sell(&final_ring[2], balance_qty, prices[2]) {
        Ok(answer) => order_result = polling_order(&account, answer.order_id, balance_qty, &final_ring[2]),
        Err(e) => println!("Error: {:?}", e),
    }
    balance_qty = get_balance(&account, &ring_component.stablecoin).unwrap();
    if order_result { println!("executed LIMIT_SELL {:?} {}", balance_qty, &final_ring[2]) }
    else { return }
}



//     println!("Which symbol to buy ? ");

//     let mut symbol:String = String::new();
//     std::io::stdin()
//         .read_line(&mut symbol)
//         .ok()
//         .expect("invalid symbol !");


