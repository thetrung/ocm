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

pub fn init_threads(rings: HashMap<String, Vec<String>>){

    // CONSTANT
    const DELAY_INIT: time::Duration = time::Duration::from_millis(100);
    const DELAY_ROUND: time::Duration = time::Duration::from_millis(1000*5);
    //
    // THREADS
    //
    let mut hunter_pool:Vec<JoinHandle<(Option<String>)>> = vec![];

    for ring in rings {
        thread::sleep(DELAY_INIT);
        let thread_ring = ring.1.clone();
        let thread = thread::spawn(|| { order_chain(ring.0, thread_ring) });
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
    println!("> RailGun Out.");
}

pub fn order_chain(symbol: String, ring: Vec<String>) -> Option<String> {
    //
    // config file loading
    //
    let mut config = Ini::new();
    let _ = config.load("config.toml");
    let market = get_market(&mut config);

    // collect prices vector
    let mut ring_prices = vec![];
    //
    // collect prices :
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
    for sym in ring {
        match market.get_price(sym) {
            Ok(answer) => {
                // println!("{:?}", answer);
                ring_prices.push(answer.price);
            },
            Err(e) => println!("Error: {}", e),
        }
    }
    // calculate if it's profitable :
    let invest = 5000.0; // x15 Max Parallel Executors each round (?)
    let binance_fees = 0.1; // as 0.1 ~ 0.0750% 
    let warning_ratio = 20.0; // 1:20 ratio
    let fees = 1.0 - (binance_fees / 100.0); 
    let sum = ( invest / ring_prices[0] ) * ring_prices[1] * ring_prices[2];
    let profit = (sum * fees * fees * fees ) - invest;
    //
    // INVALID
    //
    if profit > invest / warning_ratio { 
        println!("{}\n- FINISH -\n\n","WARNING: REMOVE THIS PAIR".red());
        return None;
        // return Some(ring_name); // in case, we need to ignore;
    }
    //
    // OK
    //
    if profit > 0.0 {

        println!("{}",
            format!("ring: {} > {} > {}",
            ring_prices[0], ring_prices[1], ring_prices[2]).to_string().cyan());

        println!("{} = {} / {:.5}%\n\n", 
            &symbol.bold(), 
            (&profit).to_string().green(), 
            ((&profit/&invest)*100.0).to_string().yellow());

        return Some(symbol);
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