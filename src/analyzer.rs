use std::{
    thread::{self, JoinHandle} ,
    collections::HashMap, 
    time::{Duration, SystemTime}};

use colored::*;
use configparser::ini::Ini;

use binance::errors::ErrorKind as BinanceLibErrorKind;
use binance::{api::*, model::{Prices, SymbolPrice}};
use binance::account::*;
use binance::market::*;
use binance::config::*;

use crate::exchangeinfo::QuantityInfo;

mod executor;
// TODO:
// 1. Optimize data vs. execution
// S
//
//
const IS_DEBUG:bool = false;
const MAX_INVEST:f64 =50.0;//368.18;
const SYMBOL_CACHE_FILE:&str = "symbols.cache";
const DELAY_INIT: Duration = Duration::from_millis(2000);

pub struct RingResult {
    symbol :String,
    percentage :f64, 
    profit :f64, 
    qty :f64, 
    optimal_invest :f64
}

pub struct RingComponent {
    symbol: String,
    bridge: String,
    stablecoin: String
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
            match cache_file.write(SYMBOL_CACHE_FILE) {
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
    tickers_buy: &mut HashMap<String, [f64;2]>, 
    tickers_sell: &mut HashMap<String, [f64;2]>,
    quantity_info: &HashMap<String, QuantityInfo>
 ) -> bool {
    //
    // update orderbooks 
    //
    match market.get_all_book_tickers() {
        Ok(answer) => {
            match &answer {
                binance::model::BookTickers::AllBookTickers(all) => {
                    match all {
                        tickers => {
                            // took 600ms ~ 290ms to fetch all tickers
                            for ticker in tickers {
                                // add only ring symbols
                                if symbol_caches.contains(&ticker.symbol) {
                                    // let step_price = quantity_info[&ticker.symbol].step_price;
                                    // let new_bid_price = correct_price_filter(&ticker.symbol, quantity_info, ticker.bid_price + step_price);
                                    // let new_ask_price = correct_price_filter(&ticker.symbol, quantity_info, ticker.ask_price - step_price);
                                    // tickers_buy.entry(ticker.symbol.clone()).or_insert([new_bid_price, ticker.bid_qty]);
                                    // tickers_sell.entry(ticker.symbol.clone()).or_insert([new_ask_price, ticker.ask_qty]);
                                    tickers_buy.entry(ticker.symbol.clone()).or_insert([ticker.bid_price, ticker.bid_qty]);
                                    tickers_sell.entry(ticker.symbol.clone()).or_insert([ticker.ask_price, ticker.ask_qty]);
                                }
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
pub fn analyze_ring( symbol: String, _ring: Vec<String>, min_invest: f64,
    tickers_buy: HashMap<String, [f64;2]>, tickers_sell: HashMap<String, [f64;2]>) -> Option<RingResult> {
    //
    // IMPORTANT: 
    // we try to buy the best bid/ask but it really depends on strategy here.
    // reverse this behaviour and the profit trade is different.
    // if we use orderbook of sell/buy/buy for BUY-SELL-SELL sequence,
    // there will be almost no chance to gain profit, because :
    // - sell : will be lowest but higher than average price
    // - buy : will be highiest but lower than average price 
    //
    let ring_prices = build_ring(&_ring, &tickers_buy, &tickers_sell);
    //
    // calculate if it's profitable :
    //
    let binance_fees = 0.1; // as 0.1 ~ 0.0750% 
    let warning_ratio = 5.0; // as ~ 5.0%
    let fees = 1.0 - (binance_fees / 100.0); 

    let max_invest = MAX_INVEST; // note : 1 round = { 3 trades + 1 request } per second is goal.
    let optimal_invest = if min_invest > max_invest { max_invest } else { min_invest };
        
    let sum = ( optimal_invest / ring_prices[0][0] ) * ring_prices[1][0] * ring_prices[2][0];
    let profit = (sum * fees * fees * fees ) - optimal_invest;
    // let profit = sum - optimal_invest;
    //
    // OK
    // let's say, we only accept profit > 0.25%
    if profit > (0.5/100.0) * optimal_invest {
        let qty = optimal_invest / ring_prices[0][0];       // println!("optimal / price {} = {}", symbol ,qty);
        let percentage = (profit/optimal_invest)*100.0;     // Ranking w/ Profit
        // LOG
        let ring_details = format!("{:?} > {:?} > {:?}", ring_prices[0], ring_prices[1], ring_prices[2]).to_string().cyan();
        let log_profit = format!("{:.5}{} {} ${:.4} max: ${:4.2}\t | {}", 
        (&percentage).to_string().yellow(), "%".yellow(), "=".bold(), (&profit).to_string().green(), &optimal_invest, &symbol.bold());
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

fn compute_rings(rings: &HashMap<String, Vec<String>>, balance: f64,
    tickers_buy: &HashMap<String, [f64;2]>, tickers_sell: &HashMap<String, [f64;2]>) -> Vec<RingResult>{
    //
    // THREADPOOL
    //
    let mut compute_pool:Vec<JoinHandle<Option<RingResult>>> = vec![];

    for ring in rings {
        // copying data :
        let symbol = ring.0.clone(); // coin name
        let _ring = ring.1.clone();  // ring of pairs
        let _tickers_buy = tickers_buy.clone();
        let _tickers_sell = tickers_sell.clone();
        let _balance = balance.clone();
        // spawn computation          
        let thread = thread::spawn(move || { analyze_ring(symbol, _ring, _balance, _tickers_buy, _tickers_sell) });
        compute_pool.push(thread);
    }
    let mut round_result = vec![];
    for computer in compute_pool {
        match computer.join().unwrap() {
            Some(result) => round_result.push(result),
            None => {}
        }
    }
    if IS_DEBUG { println!("> result: {} profitable rings", round_result.len()); }
    return round_result;
}

pub fn init_threads(config: &Ini, market: &Market, symbols_cache: &Vec<String>, 
    rings: HashMap<String, Vec<String>>, quantity_info: &HashMap<String, QuantityInfo>){
    //
    // ACCOUNT
    let account = get_account(config);
    //
    // RING COMPONENTS
    //
    let _config_bridges = &config.get("symbols", "bridges").unwrap();
    let config_bridges:Vec<&str> = _config_bridges.split(',').collect();
    let mut ring_component = RingComponent {
        symbol: String::from(""),
        bridge: String::from(config_bridges[1]),
        stablecoin: String::from(config_bridges[0]) 
    };
    let mut virtual_account = executor::get_balance( &account, &ring_component.stablecoin).unwrap();

    println!("> searching...");
    //
    // BLOCK COUNT
    //
    let mut block_count = 0;
    loop {
        println!("\n> ===================[ Block {} ]=================== <", block_count.to_string().yellow());
        let benchmark = SystemTime::now();  // BENCHMARK
        let mut tickers_update_time:Duration = Duration::from_millis(0);
        let mut tickers_buy: HashMap<String, [f64;2]> = HashMap::new();
        let mut tickers_sell: HashMap<String, [f64;2]> = HashMap::new();
        match update_orderbooks(&market, &symbols_cache, &mut tickers_buy, &mut tickers_sell, &quantity_info) {
            true => { // Check time : 
                match benchmark.elapsed() {
                    Ok(elapsed) => tickers_update_time = elapsed,
                    Err(e) => println!("> can't benchmark update_orderbooks: {:?}", e)
                }
            },
            false => return // break the loop.
        }
        // Get computed result 
        let mut round_result = compute_rings( &rings, virtual_account.clone(), &tickers_buy, &tickers_sell);
        let arbitrage_count = round_result.len();

        // If there's profitable ring 
        if arbitrage_count > 0 {
            // tickers time
            println!("{}", format!("#{}: updated orderbooks in {} ms", 
            block_count.to_string().yellow(), tickers_update_time.as_millis().to_string().yellow()));
            // Sort by Profit 
            round_result.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());
            println!("> found {} arbitrages.", arbitrage_count);
            let trade = &round_result[0];
            println!("> best: {} | {:.2}% = ${:.2}",trade.symbol, trade.percentage, trade.profit);
            if IS_DEBUG {
                println!();
                println!("____________________________");
                for result in &round_result {
                    println!("| {:.2}% = ${:.2}    | {}",result.percentage, result.profit, result.symbol);
                }
                println!("____________________________");
            }
            // Build ring prices
            let final_ring = &rings[&trade.symbol];
            let ring_prices = build_ring(final_ring, &tickers_buy, &tickers_sell);

            // 2. send it > executor
            ring_component.symbol = trade.symbol.clone(); 
            println!("> best: {} > {} > {}", ring_component.symbol, ring_component.bridge, ring_component.stablecoin);
            println!("> best: buy {} > sell {} > sell {}", ring_prices[0][0], ring_prices[1][0], ring_prices[2][0]);
            let new_balance = executor::execute_final_ring(&account, &ring_component, final_ring, &ring_prices, trade.optimal_invest, quantity_info);
            // 3. wait for trade finish
            // 4. evaluate profit
            match new_balance {
                Some(balance) => if balance > 0.0 { virtual_account = balance } else {},
                None => { // Quit Loop because there is error. 
                    return; 
                }
            } 
            //
            // benchmark every block 
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
        //5. next block !
        block_count += 1;
        } 
        else if IS_DEBUG { println!("> no arbitrage chances."); }
        // BLOCK-TIME
        thread::sleep(DELAY_INIT);         
    }
    // ending
    println!("\n> RailGun Out.\n");
}

//
// UTILITIES
//
fn correct_price_filter(symbol: &str, quantity_info: &HashMap<String, QuantityInfo>,  price: f64) -> f64 {
    let move_price = quantity_info[symbol].move_price;
    return f64::trunc(price  * move_price) / move_price;
}

/// Build buy-sell-sell vec![ prices , qty ] for a loopring.
fn build_ring(ring: &Vec<String>, tickers_buy: &HashMap<String, [f64;2]>, tickers_sell: &HashMap<String, [f64;2]>) -> Vec<[f64;2]>{

    // limit order strategy
    let p1 = tickers_buy.get(&ring[0]).unwrap(); // LIMIT_BUY
    let p2 = tickers_sell.get(&ring[1]).unwrap(); // LIMIT_SELL
    let p3 = tickers_sell.get(&ring[2]).unwrap(); // LIMIT_SELL

    // ticker average strategy
    // let p11 = tickers_buy.get(&ring[0]).unwrap(); 
    // let p12 = tickers_sell.get(&ring[0]).unwrap(); 
    // let p1 = &[(p11[0]+p12[0])/2.0, p12[1]];

    // let p21 = tickers_buy.get(&ring[1]).unwrap(); 
    // let p22 = tickers_sell.get(&ring[1]).unwrap(); 
    // let p2 =  &[(p21[0]+p22[0])/2.0, p22[1]];

    // let p31 = tickers_buy.get(&ring[2]).unwrap(); 
    // let p32 = tickers_sell.get(&ring[2]).unwrap(); 
    // let p3 =  &[(p31[0]+p32[0])/2.0, p32[1]];

    // market order strategy
    // let p1 = tickers_sell.get(&ring[0]).unwrap(); // LIMIT_BUY
    // let p2 = tickers_buy.get(&ring[1]).unwrap(); // LIMIT_SELL
    // let p3 = tickers_buy.get(&ring[2]).unwrap(); // LIMIT_SELL

    // return values
    return vec![*p1, *p2, *p3];
}

pub fn get_market(config: &mut Ini) -> Market {
    let mainnet = Config::default();//.set_rest_api_endpoint("https://testnet.binance.vision");
    return Binance::new_with_config(
        Some(config.get("keys", "api_key").unwrap()), 
        Some(config.get("keys", "secret_key").unwrap()),
        &mainnet);
}

pub fn get_account(config: &Ini) -> Account {
    let mainnet = Config::default();//.set_rest_api_endpoint("https://testnet.binance.vision");
    return Binance::new_with_config(
        Some(config.get("keys", "api_key").unwrap()), 
        Some(config.get("keys", "secret_key").unwrap()),
        &mainnet);
}