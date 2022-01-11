use binance::api::*;
use binance::account::*;
use configparser::ini::Ini;

use std::{
    collections::HashMap,
    thread::{self, JoinHandle} ,
    time::{Duration, SystemTime}};

use crate::exchangeinfo::QuantityInfo;

/// Wait time between orders
const POLLING_ORDER: Duration = Duration::from_millis(500);

pub fn execute_final_ring(account: &Account, 
    final_ring: &Vec<String>, final_ring_prices: &Vec<f64>, 
    optimal_invest: f64, quantity_info: &HashMap<String, QuantityInfo>){
    //
    // Buy > Sell > Sell
    //
    let first_buy = final_ring[0].clone().to_string();
    let decimal_place = quantity_info[&first_buy].stepSizeDecimal as f64;

    let one_decimal:f64 = 10.0;
    let move_decimal = one_decimal.powf(decimal_place);
    let fees = 1.0 - (0.075 / 100.0);
    let buy_qty = optimal_invest/final_ring_prices[0] * fees;
    let lots_qty = f64::trunc(buy_qty  * move_decimal) / move_decimal;
    
    println!("> qty: {} => {} as {} has only {} decimals.", 
    buy_qty, lots_qty, quantity_info[&first_buy].stepSize ,decimal_place);
    // return;

    //
    // Note: 
    // Ok, situation here is, once we submit a buy order, 
    // we need to wait until the order is filled before calling the next one.
    // 
    // 1. Execute Order
    // 2. Polling orderStatus until it's filled
    // 3. Execute next order
    // 4. Repeat the whole ring
    //
    match account.limit_buy(&first_buy, lots_qty, final_ring_prices[0]) {
        Ok(answer) => {
            println!("> new order_id {:?} for {} {}", &answer.order_id, lots_qty, &first_buy);
            let order_id = answer.order_id;
            loop {
                thread::sleep(POLLING_ORDER);

                match account.order_status(&first_buy, order_id) {
                    Ok(answer) => {
                        match answer.status.as_str() {
                            "FILLED" => {
                                println!("> executed qty: {}", answer.executed_qty);
                                break;
                            },  // can move on next symbol
                            "CANCELED" => {
                                break; 
                            },// on purpose ;) move to next round ?
                            _ => println!("> {} {} is {:?}", lots_qty, &first_buy ,answer.status)
                        }
                    },
                    Err(e) => println!("Error: {:?}", e),
                }
            }            
        },
        Err(e) => println!("Error: {:?}", e),
    }
    println!("> bought successfully.");
    return;

    match account.limit_sell(&final_ring[1], 0, final_ring_prices[1]) {
        Ok(answer) => {
            println!("> sell {:?} {}", answer.executed_qty, &final_ring[1]);

           
        },
        Err(e) => println!("Error: {:?}", e),
    }

    match account.limit_sell(&final_ring[2],0, final_ring_prices[2]) {
        Ok(answer) => {            
            println!("> sell {:?} {}", answer.executed_qty, &final_ring[2]);
        },
        Err(e) => println!("Error: {:?}", e),
    }       

    // match account.market_buy("WTCETH", 5) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    match account.get_balance("BUSD") {
        Ok(answer) => println!("> Balance: ${}", answer.free),
        Err(e) => {
            println!("{:?}", e);
            // error::error_handler(e);
        },
    }
    // match account.get_account() {
    //     Ok(answer) => println!("{:?}", answer.balances),
    //     Err(e) => println!("Error: {}", e),
    // }

    // match account.get_account() {
    //     Ok(answer) => println!("{:?}", answer.balances),
    //     Err(e) => println!("{:?}", e),
    // }

    // match account.limit_buy("WTCETH", 10, 0.014000) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.market_buy("WTCETH", 5) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.limit_sell("WTCETH", 10, 0.035000) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.market_sell("WTCETH", 5) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.custom_order("WTCETH", 9999, 0.0123, "SELL", "LIMIT", "IOC") {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }


    // let order_id = 1_957_528;
    // match account.order_status("WTCETH", order_id) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.get_open_orders("WTCETH") {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.cancel_order("WTCETH", order_id) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.cancel_all_open_orders("WTCETH") {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

}



//     println!("Which symbol to buy ? ");

//     let mut symbol:String = String::new();
//     std::io::stdin()
//         .read_line(&mut symbol)
//         .ok()
//         .expect("invalid symbol !");


