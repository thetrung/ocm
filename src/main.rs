extern crate binance;

use std::{thread::{self, JoinHandle}, time};

use colored::*;
use configparser::ini::Ini;

use binance::{api::*, model::{Prices, SymbolPrice}};
use binance::market::*;
use binance::account::*;
// NOTE:
//
// we will use this to finish the ring with 3 parts : 
// BUSD >> ETHBUSD > ETHBNB > BNBBUSD >> BUSD
//
// which mean each ring need to compose like : 
// 1. symbol + BUSD 
// 2. symbol + BNB
// then combine : 1 > 2 > 1
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
    let market = get_market(&mut config);
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
    //
    // LOG-TEST
    //
    // println!("{}-{}",data_ignored_symbols, ignored_symbols.len());
    // println!("{}-{}",data_bridge_symbols, bridges_symbols.len());
    // for bridge in &bridges_symbols {
    //     println!("{}", bridge);
    // }
    // for ignore in &ignored_symbols {
    //     println!("{}", ignore);
    // }
    
    let rings = symbol_discovery(&market, &ignored_symbols, &bridges_symbols, &mut data_cache);

    //
    // THREADS
    //
    let mut hunter_pool:Vec<JoinHandle<(Option<String>)>> = vec![];

    for ring in rings {
        thread::sleep(DELAY_INIT);
        let thread_ring = ring.clone();
        let thread = thread::spawn(|| { order_chain(thread_ring) });
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

    // Order book at depth 500
    // match market.get_depth("BNBBUSD") {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {}", e),
    // }
}

fn order_chain(ring: [String;3]) -> Option<String> {
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

// TODO:
// - Save discovered pairs to a cache file
// - Only re-run cache if no cache file is found, or forced update.
// * This should be done to reduce function call and init time.
//
//
// 'a is to fix the damn "named lifetime parameter" to indicate same data flow
fn symbol_discovery<'a>(
    market: &Market,
    symbols_except: &Vec<&str>, 
    symbols_bridge: &Vec<&str>,
    data_cache: &'a mut Vec<SymbolPrice>) -> Vec<[String; 3]> {

    //
    // INIT
    //
    let mut symbols_BUSD:Vec<&str> = vec![];
    let mut symbols_BNB:Vec<&str> = vec![]; 

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
                                symbols_BUSD.push(key); 
                                // println!(">> BUSD: {}", key);
                            }
                            else if key.ends_with("BNB"){
                                symbols_BNB.push(key); 
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

    println!("Total bridge-able BNB symbols is {}", symbols_BNB.len());
    println!("Total bridge-able BUSD symbols is {}", symbols_BUSD.len());

    let mut symbols_rings = vec![];

    for sym in symbols_BUSD {
        let name = String::from(sym.strip_suffix("BUSD").unwrap());
        let bridge = [name.clone(), "BNB".to_string()].join("");
        if symbols_BNB.contains(&bridge.as_str()) {
            // println!("converting {} to {} for {}", &sym, &name.as_str(), &bridge.as_str());
            //
            // Note:
            // Remember to clone/new String to copy/concat around.
            //
            symbols_rings.push([
                String::from(sym), 
                bridge.clone(),     // clone bridge.
                "BNBBUSD".to_string()
            ]);
        }
        // println!("{}-{}-{}", sym, bridge, "BNBBUSD"); // name+bridge moved here
    }

    println!("Total symbol rings is {}\n", symbols_rings.len());

    return symbols_rings;
}

fn get_str<'a>(config: &mut Ini, key: &str) -> Option<String> {
    let conf = Some(config.get("keys", key).unwrap());
    return conf;
}
fn get_market(config: &mut Ini) -> Market {
    let secret_key = get_str(config, "secret_key");
    let api_key = get_str(config, "api_key");
    let market: Market = Binance::new(api_key, secret_key);
    return market;
}

// fn buy_symbol_with_btc<S>(market: Market, account: Account) 
// where S: Into<String> 
// {
//     println!("Which symbol to buy ? ");

//     let mut symbol:String = String::new();
//     std::io::stdin()
//         .read_line(&mut symbol)
//         .ok()
//         .expect("invalid symbol !");

//     // convert to String to borrow later
//     let _symbol:String = symbol.into();

//     // Latest price for ONE symbol
//     match market.get_price(&_symbol) {
//         Ok(answer) => {
//             println!("\n- {}: {}", answer.symbol, answer.price);
//             let current_price = answer.price;

//             // get all BTC 1st 
//             match account.get_balance("BTC") {
//                 Ok(answer) => {
//                     println!("- BTC free: {}", answer.free);
//                     // "balances": [
//                     // {
//                     //     "asset": "BTC",
//                     //     "free": "4723846.89208129",
//                     //     "locked": "0.00000000",
               // },
//                     let available_btc:f64 = answer.free.parse().unwrap();
//                     let qty = &available_btc / &current_price;
//                     //
//                     // we convert all current BTC into the next coin:
//                     //

//                     println!("- market_buy {} {}", qty ,_symbol);

//                     // buy all with btc 
//                     match account.market_buy(&_symbol, qty) {
//                         Ok(answer) => {
//                             println!("- success => {:?}\n", answer)
//                         },
//                         Err(e) => println!("- ERROR: \n{:?}", e),
//                     }
//                 },
//                 Err(e) => println!("Error: {:?}", e),
//             }
//         },
//         Err(e) => println!("Error: {:?}", e),
//     }

//     println!("\n");
// }

