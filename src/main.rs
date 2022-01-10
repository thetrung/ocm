extern crate binance;

use std::{
    thread::{self, JoinHandle} ,
    collections::HashMap, 
    time};

use colored::*;
use configparser::ini::Ini;

use binance::{api::*, model::{Prices, SymbolPrice}};
use binance::market::*;

mod analyzer;
// NOTE:
//
// 1. Analyze
// Discover all trade-able symbols and serialize to cache on 1st run.
// * Could make a filtering phrase by testing run to filter old/unused pairs.
//
// 2. Update Orderbooks
// Update price by bookTickers to cache + Calculate profit through each ring.
// 
// 3. Execute Order
// Use Market Buy/Sell as fast as possible when profit is positive.
// 
// 4. Prepare for next Block
// Each block last for 10 seconds to avoid binance warn : 
// - Update OrderBooks
// - Calculate profit 
// - Execute Order
// - Eval 
// - Wait next block
//
fn main() {
    // CONSTANT
    const DELAY_INIT: time::Duration = time::Duration::from_millis(100);
    const DELAY_ROUND: time::Duration = time::Duration::from_millis(1000*5);

    //
    // CONFIG 
    //
    let mut config = Ini::new();
    let _ = config.load("config.toml");
    let market = analyzer::get_market(&mut config);
    // 
    // All we need is this tickers to divide into ASK+BID table
    // to cache prices for each symbol before computing profitable chances.
    // 
    //
    // match market.get_all_book_tickers() {
    //     Ok(answer) => {
    //         match &answer {
    //             binance::model::BookTickers::AllBookTickers(all) => {
    //                 match all {
    //                     tickers => {
    //                         for ticker in tickers {
    //                             println!("{}: ask {} x {} === bid {} x {}", 
    //                             ticker.symbol,    // Symbols
    //                             ticker.ask_price, // Sell orders
    //                             ticker.ask_qty,
    //                             ticker.bid_price, // Buy orders
    //                             ticker.bid_qty);
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //         // println!("{:?}", &answer);
    //     },
    //     Err(e) => println!("Error: {}", e),
    // }
    // return;
    //
    // BUILD RINGS
    //
    let data_ignored_symbols = &config.get("symbols","ignored").unwrap();
    let data_bridge_symbols = &config.get("symbols","bridges").unwrap();
    //
    // Transform data :
    //
    let ignored_symbols:Vec<&str> = data_ignored_symbols.split(',').collect();
    let bridges_symbols:Vec<&str> = data_bridge_symbols.split(',').collect();
    let mut data_cache:Vec<SymbolPrice> = vec![];
    
    let rings = analyzer::symbol_discovery(&market, &ignored_symbols, &bridges_symbols, &mut data_cache);

    // return;

    //
    // THREADS
    //
    let mut hunter_pool:Vec<JoinHandle<(Option<String>)>> = vec![];

    for ring in rings {
        thread::sleep(DELAY_INIT);
        let thread_ring = ring.1.clone();
        let thread = thread::spawn(|| { analyzer::order_chain(thread_ring) });
        hunter_pool.push(thread);
    }
    //
    // Joining all
    //
    let mut round_result = vec![];
    for hunter in hunter_pool{
        match hunter.join().unwrap() {
            Some(name) => {
                // make a list of ring:
                round_result.push(name);
                //
                // Plan :
                // We may rank the most profit one from each round,
                // then execute the order ring as fast as possible.
                // benchmark the delay + order book.
                //
                // Analyze > Ranking > Execute > Evaluate > Sleep > Repeat
                //
            },
            None => {

            }
        }
    }
    thread::sleep(DELAY_ROUND);
    //
    // later, we may use signal to trigger this
    //
    println!("> Joining all threads.. <");
    println!("=== This Round result ===");

    for name in round_result {
        println!("\"{}\"",name);
    }

    // ending
    println!("Ended all threads.");

}

