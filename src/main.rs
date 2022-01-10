extern crate binance;

use std::{ thread::{self, JoinHandle}, time};

use colored::*;
use configparser::ini::Ini;

use binance::{api::*, model::{Prices, SymbolPrice}};

mod analyzer;
// NOTE:
//
// 1. Analyze
// [done] Discover all trade-able symbols and serialize to cache on 1st run.
// [*] Could make a filtering phrase by testing run to filter old/unused pairs.
//
// 2. Update Orderbooks
// [wip] Update price by bookTickers to cache + Calculate profit through each ring.
// 
// 3. Execute Order
// [*] Use Market Buy/Sell as fast as possible when profit is positive.
// 
// 4. Prepare for next Block
// [*] Each block last for 10 seconds to avoid binance warn : 
// - Update OrderBooks
// - Calculate profit 
// - Execute Order
// - Eval 
// - Wait next block
//
fn main() {
    //
    // CONFIG 
    //
    let mut config = Ini::new();
    let _ = config.load("config.toml");
    let market = analyzer::get_market(&mut config);
    //
    // BUILD RINGS
    //
    let rings = analyzer::symbol_discovery(&config, &market);
    //
    // UPDATE PRICES
    //
    analyzer::init_threads(rings);
}

