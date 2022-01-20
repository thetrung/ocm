
use colored::*;
use configparser::ini::Ini;

use binance::api::*;
use binance::account::*;
use binance::market::Market;
use binance::model::Transaction;
use binance::errors::ErrorKind as BinanceLibErrorKind;

use std::{
    collections::HashMap,
    thread::{self, JoinHandle} ,
    time::{Duration, SystemTime}};

use crate::exchangeinfo::QuantityInfo;
use crate::analyzer::RingComponent;
use crate::analyzer::IS_TESTING;

/// counting before dropping an ongoing order.
const DROP_ORDER:i32 = 3;
const DROP_ORDER_PARTIAL:i32 = 6;
const MIN_SHORT_SELLING_PROFIT:f64 = 0.01;

/// Wait time between orders
const POLLING_ORDER: Duration = Duration::from_millis(500);
// const POLLING_ORDER_WAIT: Duration = Duration::from_millis(1000);

/// Poll and Wait until an order is filled.
fn polling_order(account: &Account, market: &Market, order_id: u64, qty: f64, symbol: &str, origin_balance: f64, 
    final_ring: &Vec<String>, ring_component: &RingComponent, quantity_info: &HashMap<String,QuantityInfo>,
    is_1st_order: bool, is_short_selling: bool) -> Option<f64> {
    let mut polling_count = 0;
    println!("> order: #{} for {} {}", &order_id.to_string().yellow(), qty.to_string().green(), &symbol.green());
    loop {
        match account.order_status(symbol, order_id) {
            Ok(answer) => {
                match answer.status.as_str() {
                    "FILLED" => {
                        println!("> executed qty: {}/{} after {} polls.", answer.executed_qty.green(), qty, &polling_count);
                        return Some(answer.executed_qty.parse::<f64>().unwrap());
                    },  // can move on next symbol
                    "CANCELED" => return None, // on purpose ;) move to next round ?
                    "NEW" => if polling_count > DROP_ORDER && is_1st_order { 
                        match account.cancel_order(symbol, order_id){
                            Ok(_) => { 
                                println!("> cancelled #{} after {} polls.", order_id.to_string().yellow(), polling_count);
                                return None; // cancel and re-buy like market-buy.
                            },
                            Err(e) => format_error(e.0)
                        }
                    } else if polling_count > DROP_ORDER && !is_1st_order && !is_short_selling { 
                        // NOTE: SELL NOW IF PROFITABLE 
                        // * we can sell current asset if it's profitable
                        //
                        let tickers_symbol = get_tickers_for(&market, &final_ring[0]);
                        let symbol_qty = answer.orig_qty.parse::<f64>().unwrap();
                        let sum = symbol_qty * tickers_symbol[1]; // if we short-sell by ask price.
                        let profit = sum - origin_balance;
                        if profit > MIN_SHORT_SELLING_PROFIT {
                            println!("> new: waited {} polls >> sell now for {}", polling_count, profit);
                            match account.cancel_order(symbol, order_id){
                                Ok(_) => { 
                                    println!("> cancelled #{} after {} polls.", order_id.to_string().yellow(), polling_count);
                                    match account.limit_sell(&final_ring[0], symbol_qty, tickers_symbol[1]) {
                                        Ok(result) => {
                                            polling_order(&account, &market, result.order_id, qty, &final_ring[0], origin_balance, &final_ring, &ring_component, &quantity_info, false, true);
                                            let _balance = get_balance(&account, &ring_component.stablecoin).unwrap();
                                            println!("> sold {} {} for {:.2}", result.executed_qty, &final_ring[0].green(), _balance - origin_balance);
                                            return None;
                                        },
                                        Err(e) => format_error(e.0)
                                    }
                                    return None; // cancel and re-buy like market-buy.
                                },
                                Err(e) => format_error(e.0)
                            }
                        } //else { println!("> new: not profitable for short-selling {:.2}", profit) }
                    },
                    "PARTIALLY_FILLED" => { //WARNING: NOT TESTED
                        // SELL PARTIAL FILLED ASSET 
                        // - if wait for too long + profitable.
                        //
                        if !is_1st_order && polling_count > DROP_ORDER_PARTIAL {
                            // get remaining qty
                            let symbol_asset = correct_price_filter(&final_ring[0], &quantity_info, 
                            answer.orig_qty.parse::<f64>().unwrap() - answer.executed_qty.parse::<f64>().unwrap()); 
                            // with new bridge qty
                            let bridge_asset = get_balance(&account, &ring_component.bridge).unwrap();
                            // update prices
                            let tickers_symbol = get_tickers_for(&market, &final_ring[0]);
                            let tickers_bridge = get_tickers_for(&market, &final_ring[2]);
                            let sum = symbol_asset * tickers_symbol[1] + bridge_asset * tickers_bridge[1]; // to sell fast
                            let profit = sum - origin_balance;
                            if profit > MIN_SHORT_SELLING_PROFIT {
                                println!("> partial_filled: waited {} polls >> sell now for {}", polling_count, profit);

                                match account.limit_sell(&final_ring[0], symbol_asset, tickers_symbol[1]) {
                                    Ok(result) => {
                                        polling_order(&account, &market, result.order_id, symbol_asset, &final_ring[0], origin_balance, &final_ring, &ring_component, &quantity_info, false, true);
                                        println!("> partial_filled: sold {} {}", result.executed_qty, &final_ring[0]);

                                        match account.limit_sell(&final_ring[2], bridge_asset, tickers_bridge[1]) {
                                            Ok(result) => {
                                                polling_order(&account, &market, result.order_id, symbol_asset, &final_ring[0], origin_balance, &final_ring, &ring_component, &quantity_info, false, true);
                                                println!("> partial_filled: sold {} {}", result.executed_qty, &final_ring[2]);
                                                return None; // break now.
                                            },
                                            Err(e) => format_error(e.0)
                                        }
                                    },
                                    Err(e) => format_error(e.0)
                                }
                            } else { println!("> it's not profitable to sell now: {}", sum - origin_balance) }
                        }
                    }
                    _ => {}//println!("> {} {} is {:?}", qty, &symbol ,answer.status)
                }
            },
            Err(e) => format_error(e.0),
        }
        if polling_count > 0 { thread::sleep(POLLING_ORDER) }
        polling_count += 1;
    }            
}

/// return [ ask, bid ] prices of a single symbol.
fn get_tickers_for(market: &Market, symbol: &str) -> Vec<f64> {
    match market.get_book_ticker(symbol) {
        Ok(result) => return vec![result.ask_price, result.bid_price],
        Err(e) => { format_error(e.0); return vec![] }
    };
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
        "> success: {:?} {} after {} ms.\n", 
        balance_qty, 
        symbol, 
        benchmark.elapsed().unwrap().as_millis().to_string()).green());
}

/// Execute best ring found in previous round result.
pub fn execute_final_ring(account: &Account, market: &Market, ring_component: &RingComponent, final_ring: &Vec<String>, 
    prices: &Vec<[f64;2]>, config_invest: f64, quantity_info: HashMap<String, QuantityInfo>) -> Option<f64> {
    
    let benchmark = SystemTime::now();
    println!("> -------------------------------------------------- <");
    //> for testing purpose.
    if IS_TESTING { return Some(0.0); }

    // states
    let mut order_result:Option<f64> = None;

    // correct lots + step_size
    let mut symbol:&str;
    // let mut step_qty:f64 = 0.0;
    // let mut step_price:f64 = 0.0;
    let mut balance_qty:f64 = 0.0;
    let mut custom_price:f64 = 0.0;
    
    // prepare balance 
    let _current_balance = get_balance(&account, &ring_component.stablecoin).unwrap(); println!();
    if _current_balance < 10.0 { return None; } // Break because this will be serious error.
    let optimal_invest = if config_invest > _current_balance { _current_balance } else { config_invest };


    // step_qty = quantity_info[symbol].step_qty;
    // step_price = quantity_info[symbol].step_price;
    //
    // 1. Buy OOKI-BUSD
    //
    symbol = &final_ring[0];
    let first_order = optimal_invest/(prices[0][0]);

    balance_qty = correct_lots_qty(symbol, first_order, &quantity_info);
    println!("> limit_buy: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), (optimal_invest/first_order).to_string().yellow());
    match account.limit_buy(symbol, balance_qty, prices[0][0]) {
    // match account.market_buy(symbol, balance_qty) {
        Ok(answer) => order_result = polling_order(&account, &market, answer.order_id, balance_qty, symbol, _current_balance, &final_ring, &ring_component, &quantity_info, true, false),
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
    balance_qty = correct_lots_qty(symbol, balance_qty, &quantity_info); 
    custom_price = correct_price_filter(symbol, &quantity_info, prices[1][1]);
    println!("> limit_sell: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[1][1].to_string().yellow());
    match account.limit_sell(symbol, balance_qty, custom_price) {
        Ok(answer) => order_result = polling_order(&account, &market, answer.order_id, balance_qty, symbol, _current_balance, &final_ring, &ring_component, &quantity_info, false, false),
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
        None => return Some(-1.0)  // None can help to break + stop App.
    }

    //
    // 3. Sell BTC-BUSD
    //
    symbol = &final_ring[2];
    balance_qty = correct_lots_qty(symbol, balance_qty, &quantity_info);
    println!("> limit_sell: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[2][1].to_string().yellow());
    match account.limit_sell(symbol, balance_qty, prices[2][1]) {
    // match account.market_sell(symbol, balance_qty) {
        Ok(answer) => order_result = polling_order(&account, &market, answer.order_id, balance_qty, symbol, _current_balance, &final_ring, &ring_component, &quantity_info, false, false),
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
        None => return Some(-1.0) // None can help to break + stop App. 
    }

    return Some(balance_qty);
}

/// Execute best ring found in previous round result.
pub fn execute_final_ring_pallarel(account: &Account, market: &Market, ring_component: &RingComponent, final_ring: &Vec<String>, 
    prices: &Vec<[f64;2]>, config_invest: f64, quantity_info: HashMap<String, QuantityInfo>) -> Option<f64> {
    
    let benchmark = SystemTime::now();
    println!("> -------------------------------------------------- <");
    //> for testing purpose.
    // return Some(0.0);

    // states
    let mut order_result:Vec<(&str, u64)> = vec![];

    // correct lots + step_size
    let mut symbol:&str;
    // let mut step_qty:f64 = 0.0;
    // let mut step_price:f64 = 0.0;
    let mut balance_qty:f64 = 0.0;
    let mut custom_price:f64 = 0.0;
    
    // prepare balance 
    let _current_balance = get_balance(&account, &ring_component.stablecoin).unwrap(); println!();
    if _current_balance < config_invest / 5.0 { return None; } // Break because this will be serious error w/ only 20%
    let optimal_invest = if config_invest > _current_balance { _current_balance } else { config_invest };


    // step_qty = quantity_info[symbol].step_qty;
    // step_price = quantity_info[symbol].step_price;
    //
    // 1. Sell OOKI-BTC
    //
    let order_qty_a = correct_lots_qty(&final_ring[0], optimal_invest/(prices[0][0]), &quantity_info); // which result in a (symbol)
    let order_qty_b = correct_lots_qty(&final_ring[2], order_qty_a * prices[1][0], &quantity_info);    // which result in b (bridge)
    let order_qty_c = correct_lots_qty(&final_ring[0], order_qty_b * prices[2][0], &quantity_info); // which result in c (stablecoin)
    println!("> qty_result: {} -> {} -> {} -> {}", optimal_invest, order_qty_b, order_qty_a, order_qty_c);

    if IS_TESTING { return Some(0.0); }

    let mut first_order:Option<f64> = None;

    symbol = &final_ring[1];
    balance_qty = order_qty_a; 
    //
    println!("> limit_sell: {} {} at {}", &balance_qty.to_string().green(), 
    symbol.green(), &prices[1][0].to_string().yellow());
    match account.limit_sell(symbol, balance_qty, prices[1][0]) {
        Ok(answer) => {
            first_order = polling_order(&account, &market, answer.order_id, balance_qty, 
            symbol, _current_balance, &final_ring, &ring_component, &quantity_info, true, false);
            },
        Err(e) => format_error(e.0)
    }
    match first_order {
        Some(executed_qty) => {
            balance_qty = executed_qty;
            format_result(balance_qty, &ring_component.symbol, &benchmark);
        },
        None => return Some(-1.0) 
    }
    //
    // 2. Buy OOKI-BUSD
    //
    symbol = &final_ring[0];
    balance_qty = order_qty_a;
    // 
    println!("> limit_buy: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[0][0].to_string().yellow());
    match account.limit_buy(symbol, balance_qty, prices[0][0]) {
        Ok(answer) => order_result.push((symbol, answer.order_id)),
        Err(e) => { 
            format_error(e.0);
            return None
        },
    }

    //
    // 3. Sell BTC-BUSD
    //
    symbol = &final_ring[2];
    balance_qty = order_qty_b;
    println!("> limit_sell: {} {} at {}", 
    &balance_qty.to_string().green(), symbol.green(), &prices[2][0].to_string().yellow());
    match account.limit_sell(symbol, balance_qty, prices[2][0]) {
        Ok(answer) => order_result.push((symbol, answer.order_id)),
        Err(e) => { 
            format_error(e.0); 
            return None; // Error
        }
    }
    // wait till all finished.
    loop {
        let mut new_order_result = order_result.clone();
        let mut finish_count = 0;
        let mut index = 0;
        for result in &order_result {
            match account.order_status(result.0, result.1) {
                Ok(answer) => {
                    match answer.status.as_str() {
                        "FILLED" => {
                            println!("> #{} finished with {} {}.", result.1, answer.executed_qty, result.0);
                            new_order_result.remove(index);
                            finish_count+= 1;
                        },
                        _ => {}
                    };
                },
                Err(e) => format_error(e.0)
            }
            index+=1;
        }
        if finish_count == order_result.len() {break}
        else { // reset counter + sleep
            index = 0;
            finish_count = 0;
            order_result = new_order_result.clone();
            thread::sleep(POLLING_ORDER);
        }
    }
    println!("> all finished.");

    return Some(get_balance(&account, &ring_component.stablecoin).unwrap());
}