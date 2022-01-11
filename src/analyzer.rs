use std::{
    thread::{self, JoinHandle} ,
    collections::HashMap, 
    time::{Duration, SystemTime}};

use colored::*;
use configparser::ini::Ini;

use binance::errors::ErrorKind as BinanceLibErrorKind;
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
const IS_DEBUG:bool = false;
const MAX_INVEST:f64 = 368.18;
const SYMBOL_CACHE_FILE:&str = "symbols.cache";
const DELAY_INIT: Duration = Duration::from_millis(2000);

pub struct RingResult {
    symbol :String,
    percentage :f64, 
    profit :f64, 
    qty :f64, 
    optimal_invest :f64
}

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
    market: &Market, symbol_caches: &Vec<String>,
    tickers_buy: &mut HashMap<String, [f64;2]> , 
    tickers_sell: &mut HashMap<String, [f64;2]> ) -> bool {
    //
    // update orderbooks 
    //
    match market.get_all_book_tickers() {
        Ok(answer) => {
            match &answer {
                binance::model::BookTickers::AllBookTickers(all) => {
                    match all {
                        tickers => {
                            // took 1.4k ~ 521ms to fetch all tickers
                            for ticker in tickers {
                                // add only ring symbols
                                if symbol_caches.contains(&ticker.symbol) {
                                    tickers_sell.entry(ticker.symbol.clone()).or_insert([ticker.ask_price, ticker.ask_qty]);
                                    tickers_buy.entry(ticker.symbol.clone()).or_insert([ticker.bid_price, ticker.bid_qty]);
                                    // println!("ticker: {:?}", tickers_sell[&ticker.symbol.clone()]);
                                }
                                // println!("{}: ask {} x {} === bid {} x {}", 
                                // ticker.symbol,    // Symbols
                                // ticker.ask_price, // Sell orders
                                // ticker.ask_qty,
                                // ticker.bid_price, // Buy orders
                                // ticker.bid_qty);
                            }
                        }
                    }
                }
            };
            return true;
        },
        Err(e) => println!("Error: {:?}\n\n> will break the loop now.\n> RailGun out.", &e.0)
    };
    return false;
}

/// Compute profit on each ring 
pub fn order_chain( symbol: String, ring_prices: Vec<f64>, volumes: Vec<f64> ) -> Option<RingResult> {
    //
    // calculate if it's profitable :
    //
    let binance_fees = 0.1; // as 0.1 ~ 0.0750% 
    let warning_ratio = 5.0; // as ~ 5.0%
    let fees = 1.0 - (binance_fees / 100.0); 

    let max_invest = MAX_INVEST; // note : 1 round = { 3 trades + 1 request } per second is goal.
    let min_volume = volumes[0]; // already sorted [ min -> max ]
    let optimal_invest = if min_volume > max_invest { max_invest } else { min_volume };
        
    let sum = ( optimal_invest / ring_prices[0] ) * ring_prices[1] * ring_prices[2];
    let profit = (sum * fees * fees * fees ) - optimal_invest;
    //
    // OK
    //
    if profit > 0.0 {
        // the quantity we will trade ?
        let qty = optimal_invest / ring_prices[0];
        // println!("optimal / price {} = {}", symbol ,qty);
        // calculate percentage for ranking :
        let percentage = (profit/optimal_invest)*100.0;
        //
        // LOG
        //
        let ring_details = format!("{:?} > {:?} > {:?}", ring_prices[0], ring_prices[1], ring_prices[2]).to_string().cyan();
        let log_profit = format!("{:.5}{} {} ${:.4} max: ${:4.2}\t | {}", 
        (&percentage).to_string().yellow(), 
        "%".yellow(), "=".bold(),
        (&profit).to_string().green(), 
        &optimal_invest, &symbol.bold());
        //
        // WARNING: invalid pairs
        if profit > optimal_invest * (warning_ratio/100.0) {     
            if !IS_DEBUG { println!("\n{}\n{} - {}\n\n", log_profit, ring_details, "WARNING: REMOVE THIS PAIR".red()); }
            return None;
        }
        //
        // PROFITABLE: normal log
        if IS_DEBUG { println!("\n{}\n{}", log_profit, ring_details); }
        return Some(RingResult { symbol, percentage, profit, qty, optimal_invest });
    } 
    return None;
}

pub fn init_threads(market: &Market, rings: HashMap<String, Vec<String>>){
    //
    // 1.Init caches 
    //
    let mut symbols_cache: Vec<String> = vec![];
    for ring in &rings {
        for symbol in ring.1 {
            symbols_cache.push(symbol.clone());
        }
    }
    if IS_DEBUG { println!("\n> computing via {} threads..", &rings.len());}
    let mut virtual_account = MAX_INVEST;
    let mut block_count = 0;
    loop {
        //
        // BLOCK-TIME
        //
        thread::sleep(DELAY_INIT);
        let benchmark = SystemTime::now();
        //
        // THREADPOOL
        //
        let mut compute_pool:Vec<JoinHandle<Option<RingResult>>> = vec![];
        //
        // 2.Update-Compute-Trade Loop :
        //
        let mut tickers_buy: HashMap<String, [f64;2]> = HashMap::new();
        let mut tickers_sell: HashMap<String, [f64;2]> = HashMap::new();
         
        match update_orderbooks(&market, &symbols_cache, &mut tickers_buy, &mut tickers_sell) {
            true => { // Check time : 
                match benchmark.elapsed() {
                    Ok(elapsed) => {
                        println!("{}", format!("\n#{}: updated orderbooks in {} ms",
                        block_count.to_string().yellow(), elapsed.as_millis().to_string().yellow()));
                    }
                    Err(e) => println!("> can't benchmark update_orderbooks: {:?}", e)
                }
            },
            false => return // break the loop.
        }
        //
        // INIT 
        //
        if IS_DEBUG {
            println!("\n\n______________[ Round #{} ]______________", block_count);
        }
        for ring in &rings {
            let symbol = ring.0.clone();
            let _ring = ring.1;

            let p1 = tickers_buy.get(&_ring[0]).unwrap();
            let p2 = tickers_sell.get(&_ring[1]).unwrap();
            let p3 = tickers_sell.get(&_ring[2]).unwrap();

            // collect ring prices 
            let mut ring_prices = vec![];
            ring_prices.push(p1[0]);  // BUSD > BRIDGE
            ring_prices.push(p2[0]); // BRIDGE > BRIDGE
            ring_prices.push(p3[0]); // BRIDGE > BUSD
            // println!("{:?}", ring_prices);
            
            // calculate min volume 
            let mut compute_min_volume = // * in stablecoin
            vec![p1[0]*p1[1],       // priceA x qtyA
                p2[0]*p2[1]*p3[0],  // priceB x priceC x qtyB
                p3[0]*p3[1]];       // priceC x qtyC

            // Sorting price x qty 
            compute_min_volume.sort_by(|a,b| (a).partial_cmp(b).unwrap());

            // spawn computation          
            let thread = thread::spawn(|| { order_chain(symbol, ring_prices, compute_min_volume) });
            compute_pool.push(thread);
        }
        //
        // :: COLLECT RESULTS  ::
        // [*] 2. Execute the order ring as fast as possible.
        //
        let mut round_result = vec![];
        for computer in compute_pool {
            match computer.join().unwrap() {
                Some(result) => round_result.push(result),
                None => {}
            }
        }
        //
        // Sort by Profit with optimal investment.
        round_result.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());
        let trade = &round_result[0];

        if IS_DEBUG {
            println!();
            println!("____________________________");
            for result in &round_result {
                println!("| {:.2}% = ${:.2}    | {}",result.percentage, result.profit, result.symbol);
            }
            println!("____________________________");
        }
        //
        // Get Total Time for 1 round 
        //
        match benchmark.elapsed() {
            Ok(elapsed) => {
                // println!("{:?}", &traded_volume_cache);
                let fmt = format!("#{}: ${} - trade {} {} by ${} for ${:.5} in {} ms",
                block_count.to_string().yellow(), 
                format!("{:.2}", virtual_account).green(),
                format!("{:.2}", trade.qty).green(), 
                trade.symbol.green(), 
                format!("{:.2}", trade.optimal_invest).green(),
                format!("{:.2}", trade.profit).yellow(), 
                elapsed.as_millis().to_string().yellow());
                println!("{}", fmt);
            }
            Err(e) => println!("Error: {:?}", e)
        }
        // next block.
        block_count += 1;
        virtual_account += trade.profit;
    }

    // ending
    println!("\n> RailGun Out.\n");
}

//
// UTILITIES
//
pub fn get_market(config: &mut Ini) -> Market {
    return Binance::new(
        Some(config.get("keys", "api_key").unwrap()), 
        Some(config.get("keys", "secret_key").unwrap()));
}