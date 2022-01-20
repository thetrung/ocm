extern crate binance;

use configparser::ini::Ini;

pub mod exchangeinfo;
pub mod analyzer;

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
    let symbols_cache = make_symcache(&rings);
    let quantity_info = exchangeinfo::fetch(&symbols_cache).unwrap();
    // return;
    //
    // UPDATE PRICES
    //
    analyzer::init_threads(&config, &market, &symbols_cache, rings, &quantity_info);
}

fn make_symcache(rings: &std::collections::HashMap<String, Vec<String>>) -> Vec<String> {
    let mut symbols_cache: Vec<String> = vec![];
    for ring in rings {
        for symbol in ring.1 {
            symbols_cache.push(symbol.clone());
        }
    }
    return symbols_cache;
}