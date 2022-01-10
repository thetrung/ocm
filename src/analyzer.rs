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
pub fn symbol_discovery<'a>(
    market: &Market,
    symbols_except: &Vec<&str>, 
    symbols_bridge: &Vec<&str>,
    data_cache: &'a mut Vec<SymbolPrice>) -> HashMap<String, Vec<String>>{
    //
    // INIT CACHES
    //
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
                        //
                        // Build Ring :
                        //
                        // print!("\n{}: ", symbol_ring.0.to_uppercase());
                        let ring:Vec<String> = symbol_ring.1.unwrap().split(",").map(|s| s.to_string()).collect();
                        //
                        // Insert into map :
                        //
                        // for sym in &ring { print!("{} ", sym)}
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
                            // caching
                            *data_cache = data.clone();
                            
                            for symbol_price in data_cache {
                                //
                                // Filter stuffs here
                                // TODO: use HashMap to add also prices
                                let key = symbol_price.symbol.as_str();
                                let mut bridge_iter = symbols_bridge.iter();
                                // println!("key: {}", key);
                                if bridge_iter.any(|bridge_sym| {
                                    // println!("item: {} vs. {}", &bridge_sym, key);
                                    key.contains(bridge_sym) 
                                } && !symbols_except.contains(&key)) {
                                    // Passed :
                                    if key.ends_with("BUSD") {
                                        symbols_stablecoin.push(key); 
                                        // println!(">> BUSD: {}", key);
                                    }
                                    else if key.ends_with("BNB"){
                                        symbols_with_bridge.push(key); 
                                        // println!(">> sym: {}", key);
                                    } 
                                } else {
                                    // println!("ignored: {}", key);
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("Error with data_cache = {:2}\n", e),
            }
            println!("- Total bridge-paired symbols is {}", symbols_with_bridge.len());
            println!("- Total stablecoin-paired symbols is {}", symbols_stablecoin.len());

            for sym in symbols_stablecoin {
                // 1 - stablecoin
                // 2 - bridge
                let name = String::from(sym.strip_suffix(symbols_bridge[0]).unwrap());
                let bridge = [name.clone(), symbols_bridge[1].to_string()].join("");
                if symbols_with_bridge.contains(&bridge.as_str()) {
                    //
                    // Note:
                    // Remember to clone/new String to copy/concat around.
                    //
                    symbols_rings.insert(String::from(name), vec![
                        String::from(sym), 
                        bridge.clone(),     // clone bridge.
                        "BNBBUSD".to_string()
                    ]);
                }
                // println!("{}-{}-{}", sym, bridge, "BNBBUSD"); // name+bridge moved here
            }
            println!("- Total symbol rings is {}", symbols_rings.len());
            //
            // Save all discovered symbols >> cache file
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


pub fn order_chain(ring: Vec<String>) -> Option<String> {
    //
    // config file loading
    //
    let mut config = Ini::new();
    let _ = config.load("config.toml");
    let market = get_market(&mut config);
    //
    //  1. Get average price of each symbol
    //  2. Convert the ring total price into profit
    //  3. Compare and Execute Order Chains
    //
    let ring_name = ring[0].clone().replace("BUSD","");
    let mut ring_prices = vec![];
    //
    // collect prices :
    //
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
            &ring_name.bold(), 
            (&profit).to_string().green(), 
            ((&profit/&invest)*100.0).to_string().yellow());

        return Some(ring_name);
    }
    
    return None;
}

//
// UTILITIES
//

fn get_str<'a>(config: &mut Ini, key: &str) -> Option<String> {
    let conf = Some(config.get("keys", key).unwrap());
    return conf;
}
pub fn get_market(config: &mut Ini) -> Market {
    let secret_key = get_str(config, "secret_key");
    let api_key = get_str(config, "api_key");
    let market: Market = Binance::new(api_key, secret_key);
    return market;
}