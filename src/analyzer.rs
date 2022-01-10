use std::{
    thread::{self, JoinHandle} ,
    collections::HashMap, 
    time};

use colored::*;
use configparser::ini::Ini;

use binance::{api::*, model::{Prices, SymbolPrice}};
use binance::market::*;


// TODO:
// - Save discovered pairs to a cache file
// - Only re-run cache if no cache file is found, or forced update.
// * This should be done to reduce function call and init time.
//
//
// EXPLAIN:
// we will use this to finish the ring with 3 parts : 
// BUSD >> ETHBUSD > ETHBNB > BNBBUSD >> BUSD
//
// which mean each ring need to compose like : 
// 1. symbol + BUSD 
// 2. symbol + BNB
// then combine : 1 > 2 > 1
//
const SYMBOL_CACHE_FILE:&str = "symbols.cache";

// 'a is to fix the damn "named lifetime parameter" to indicate same data flow
pub fn symbol_discovery<'a>(config: &Ini, market: &Market) -> HashMap<String, Vec<String>>{
    //
    // LOAD CONFIGS
    //
    let _key_bridge = &config.get("symbols","bridges").unwrap();
    let _key_ignored = &config.get("symbols","ignored").unwrap();
    let bridge_symbols:Vec<&str> = _key_bridge.split(',').collect();
    let ignored_symbols:Vec<&str> = _key_ignored.split(',').collect();
    //
    // INIT CACHES
    //
    let mut data_cache:Vec<SymbolPrice> = vec![];
    let mut symbols_stablecoin:Vec<&str> = vec![];
    let mut symbols_with_bridge:Vec<&str> = vec![]; 
    let mut symbols_rings: HashMap<String, Vec<String>> = HashMap::new();
    //
    // FIND & LOAD CACHED FILE
    //
    let mut cache_file = Ini::new();
    match cache_file.load(SYMBOL_CACHE_FILE) {
        Ok(_) => { 
            println!("> found prev cache");
            for sym_map in cache_file.get_map() {
                for discovered in sym_map {
                    println!("> loading {} rings...", discovered.1.len());
                    for symbol_ring in discovered.1 {
                        let ring:Vec<String> = 
                        symbol_ring.1.unwrap().split(",").map(|s| s.to_string()).collect();
                        symbols_rings.insert( symbol_ring.0.to_uppercase(), ring);
                    }
                }
            }
        },
        _error => {
            println!("> can't find symbol cache >> building one now ...");
            //
            // Fetching all symbols from Binance.
            //
            match market.get_all_prices() {
                Ok(answer) => {

                    match answer {
                        // need to match the exact enum
                        Prices::AllPrices(data) => {
                            //
                            // ground caching for other hashes 
                            //
                            data_cache = data.clone();
                            
                            for symbol_price in &data_cache {
                                let key = symbol_price.symbol.as_str();
                                let mut bridge_iter = bridge_symbols.iter();
                                //
                                // Find only stablecoin/bridge pairs :
                                //
                                if bridge_iter.any(|b| key.contains(b)) && !ignored_symbols.contains(&key){
                                    // Stablecoin :
                                    if key.ends_with(bridge_symbols[0]) {
                                        symbols_stablecoin.push(&key); 
                                    }
                                    // Bridge :
                                    else if key.ends_with(bridge_symbols[1]){
                                        symbols_with_bridge.push(&key);
                                    } 
                                } 
                            }
                        }
                    }
                },
                Err(e) => println!("Error with data_cache = {:2}\n", e),
            }
            println!("- Total bridge-pairs : {}", symbols_with_bridge.len());
            println!("- Total stablecoin-pairs : {}", symbols_stablecoin.len());

            for sym in symbols_stablecoin {
                // 1 - stablecoin
                // 2 - bridge
                let name = String::from(sym.strip_suffix(bridge_symbols[0]).unwrap());
                let bridge = [name.clone(), bridge_symbols[1].to_string()].join("");
                if symbols_with_bridge.contains(&bridge.as_str()) {
                    //
                    // Note:
                    // Remember to clone/new String to copy/concat around.
                    //
                    symbols_rings.insert(String::from(name), vec![
                        String::from(sym),                              // sym => XTZ-BUSD,
                        bridge.clone(),                                 // bridge => XTZ-BNB,
                        [bridge_symbols[1], bridge_symbols[0]].join("") // bridge-stablecoin => BNB-BUSD
                    ]);
                }
            }
            println!("- Total symbol rings is {}", symbols_rings.len());
            //
            // Save all discovered symbols 
            //
            let mut cache_file = Ini::new();
            for sym in &symbols_rings {
                cache_file.set(
                    "discovered", 
                    sym.0.as_str(), 
                    Option::from(sym.1.join(",")));
            }
            match cache_file.write("symbols.cache") {
                Ok(_) => println!("> built symbols cache."),
                msg => println!("Error saving cache: {:?}", msg)
            }
        }
    }
    // Done !
    println!("> built rings map.");
    return symbols_rings;
}

/// All we need is this tickers to divide into ASK+BID table
/// to cache prices for each symbol before computing profitable chances.
fn update_orderbooks(
    market: &Market,
    symbol_caches: &Vec<String>,
    tickers_buy: &mut HashMap<String, [f64;2]> , 
    tickers_sell: &mut HashMap<String, [f64;2]> ){
    //
    // update orderbooks 
    //
    match market.get_all_book_tickers() {
        Ok(answer) => {
            match &answer {
                binance::model::BookTickers::AllBookTickers(all) => {
                    match all {
                        tickers => {
                            for ticker in tickers {
                                // println!("{}: ask {} x {} === bid {} x {}", 
                                // ticker.symbol,    // Symbols
                                // ticker.ask_price, // Sell orders
                                // ticker.ask_qty,
                                // ticker.bid_price, // Buy orders
                                // ticker.bid_qty);
                                //
                                // add only ring symbols
                                //
                                if symbol_caches.contains(&ticker.symbol) {
                                    tickers_sell.entry(ticker.symbol.clone()).or_insert([ticker.ask_price, ticker.ask_qty]);
                                    tickers_buy.entry(ticker.symbol.clone()).or_insert([ticker.bid_price, ticker.bid_qty]);
                                    // println!("ticker: {:?}", tickers_sell[&ticker.symbol.clone()]);
                                }
                            }
                        }
                    }
                }
            }
            // println!("{:?}", &answer);
        },
        Err(e) => println!("Error: {}", e),
    }
    println!("- total tickers: {}/{} on {} symbols", tickers_sell.len(), tickers_buy.len(),  &symbol_caches.len());
}

pub fn init_threads(market: &Market, rings: HashMap<String, Vec<String>>){
    //
    // CONSTANT
    //
    const DELAY_INIT: time::Duration = time::Duration::from_millis(1000);
    // const DELAY_ROUND: time::Duration = time::Duration::from_millis(1000);
    //
    // THREADPOOL
    //
    let mut compute_pool:Vec<JoinHandle<(Option<(String,f64, f64)>)>> = vec![];
    //
    // COLLECT ORDERBOOKS
    // 
    // 1.Init caches 
    //
    let mut tickers_buy: HashMap<String, [f64;2]> = HashMap::new();
    let mut tickers_sell: HashMap<String, [f64;2]> = HashMap::new();
    let mut symbols_cache: Vec<String> = vec![];
    for ring in &rings {
        for symbol in ring.1 {
            symbols_cache.push(symbol.clone());
        }
    }
    //
    // 2.Update
    //
    update_orderbooks(&market, &symbols_cache, &mut tickers_buy, &mut tickers_sell);
    //
    // INIT 
    //
    println!("\n> computing via {} threads..", rings.len());
    println!("_________________________________________");
    for ring in rings {
        // thread::sleep(DELAY_INIT);
        let symbol = ring.0.clone();
        let _ring = ring.1.clone();

        // collect ring prices 
        let mut ring_prices = vec![];
        ring_prices.push(tickers_buy.get(&_ring[0]).unwrap()[0]);  // BUSD > BRIDGE
        ring_prices.push(tickers_sell.get(&_ring[1]).unwrap()[0]); // BRIDGE > BRIDGE
        ring_prices.push(tickers_sell.get(&_ring[2]).unwrap()[0]); // BRIDGE > BUSD
        // println!("{:?}", ring_prices);
        
        // ticker
        let mut tickers:Vec<[f64;2]> = vec![];
        tickers.push(tickers_buy.get(&_ring[0]).unwrap().clone());  // BUSD > BRIDGE
        tickers.push(tickers_sell.get(&_ring[1]).unwrap().clone()); // BRIDGE > BRIDGE
        tickers.push(tickers_sell.get(&_ring[2]).unwrap().clone()); // BRIDGE > BUSD

        // Quantity :
        // println!("{:?}", tickers);
        tickers.sort_by(|[_, b], [_, y]| b.partial_cmp(y).unwrap());
        // println!("{:?}", tickers);
        
        let thread = thread::spawn(|| { order_chain(symbol, ring_prices, tickers) });
        compute_pool.push(thread);
    }
    //
    // :: COLLECT RESULTS  ::
    //
    // [ok] 1. Rank the most profit one from each round, 
    // [*] 2. Execute the order ring as fast as possible.
    // [wip] 3. Benchmark the delay
    //
    let mut round_result = vec![];
    for hunter in compute_pool{
        match hunter.join().unwrap() {
            Some(result) => round_result.push(result),
            None => {}
        }
    }
    // wait for all threads..
    thread::sleep(DELAY_INIT);
    // println!();
    println!("_________________________________________\n");
    //
    // Sorting Result :
    //
    round_result.sort_by(|(_,_,a),(_,_,b)| b.partial_cmp(a).unwrap());
    println!("\n>-------[ Round {} ]--------",0);
    println!();
    for result in round_result {
        println!("| {:.2}% = ${:.1}\t | {}",result.1, result.2, result.0);
    }
    println!("____________________________");
    // ending
    println!("\n\n> RailGun Out.\n");
}

///
/// Compute profit on each ring 
/// 
pub fn order_chain( symbol: String, ring_prices: Vec<f64>, tickers: Vec<[f64;2]> ) -> Option<(String, f64, f64)> {
    //
    // calculate if it's profitable :
    //
    let max_invest = 5000.0; // x15 Max Parallel Executors each round (?)

    let binance_fees = 0.1; // as 0.1 ~ 0.0750% 
    let warning_ratio = 20.0; // 1:20 ratio
    let fees = 1.0 - (binance_fees / 100.0); 
    let min_invest = tickers[0][0] * tickers[0][1];
    let optimal_invest = if min_invest > max_invest { max_invest } else { min_invest };
    let sum = ( optimal_invest / ring_prices[0] ) * ring_prices[1] * ring_prices[2];
    let profit = (sum * fees * fees * fees ) - optimal_invest;
    let percentage = (profit/optimal_invest)*100.0;
    //
    // OK
    //
    if profit > 0.0 {
        //
        // ring details
        //
        // println!("{}",
        //     format!("ring: {:?} > {:?} > {:?}",
        //     ring_prices[0], ring_prices[1], ring_prices[2]).to_string().cyan());
        //
        // LOG
        //
        let log_profit = format!("{:.5}{} {} ${:.4} max: ${:4.2}\t | {}", 
        (&percentage).to_string().yellow(),
        "%".yellow(),
        "=".bold(),
        (&profit).to_string().green(), 
        &optimal_invest,
        &symbol.bold());
        //
        // WARNING: invalid pairs
        //
        if profit > optimal_invest / warning_ratio {     
            println!("\n\n{} - {}\n\n",log_profit, "WARNING: REMOVE THIS PAIR".red());
            return None;
            // return Some(ring_name); // in case, we need to ignore;
        }
        //
        // PROFITABLE: normal log
        //
        println!("{}", log_profit);
        return Some((String::from(symbol), percentage, profit));
    }
    
    return None;
}

//
// UTILITIES
//
pub fn get_market(config: &mut Ini) -> Market {
    return Binance::new(
        Some(config.get("keys", "api_key").unwrap()), 
        Some(config.get("keys", "secret_key").unwrap()));
}