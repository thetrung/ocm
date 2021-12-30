extern crate binance;

use std::{collections::HashMap, thread::{self, JoinHandle}, time};

use colored::*;
use configparser::ini::Ini;

use binance::{api::*, model::{Prices, SymbolPrice}};
use binance::market::*;
use binance::account::*;

struct Hunter {
    symbol: String,
    is_riding: bool,
    heart_beat: i32, // ms
    gain: f64,       // %
    gain_min: f64,   // %
    gain_max: f64,   // %
}

struct Config {
    // keys
    api_key: String,
    secret_key: String,
    // thread interval
    init_heart_beat: i32,
    // bot config
    fix_gain_step: f64,
    init_gain_min: f64,
    init_gain_max: f64
}

const one_second: time::Duration = time::Duration::from_millis(1000);
const two_second: time::Duration = time::Duration::from_millis(2500);

fn main() {
    // caching to extend live of data
    let mut data_cache: Vec<SymbolPrice> = vec![];
    let mut config = Ini::new();
    config.load("config.toml");
    let market = get_market(&mut config);
    // let symbols = symbol_scan(&market, &mut data_cache);

    // Order book at depth 500
    match market.get_depth("BNBBUSD") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // Test 
    // let symbols = vec![
    //     "SFPUSDT", 
    //     "DODOUSDT", 
    //     "FIROUSDT", 
    //     "SANDUSDT", 
    //     "PERLUSDT"
    //     ]; // no gas
    
    // let mut hunter_pool:Vec<JoinHandle<()>> = vec![];

    // for symbol in symbols {
    //     thread::sleep(one_second);
    //     let sym_str = String::from(symbol);
    //     let thread = thread::spawn(|| { whale_scan( sym_str); });
    //     hunter_pool.push(thread);
    // }

    // //
    // // later, we may use signal to trigger this
    // //
    // println!("> Last Block before joining all threads.. <");

    // //
    // // Joining all
    // //
    // for hunter in hunter_pool{
    //     hunter.join().unwrap(); 
    // }

    // // ending
    // println!("Ended all threads.");
}

// 'a is to fix the damn "named lifetime parameter" to indicate same data flow
fn symbol_scan<'a>(market: &Market, data_cache: &'a mut Vec<SymbolPrice>) -> Vec<&'a str> {

    // TODO: use
    let mut symmap:HashMap<&str,f64> = HashMap::new();
    let mut symbols_BTC:Vec<&str> = vec![];
    let mut symbols:Vec<&str> = vec![]; // init
    let symbols_except = vec![   
        "USDSUSDT",
        "USDCUSDT",      
        "USDSBUSDT"
    ];
    //
    // Fetching all prices of symbols on Binance..
    // may call this once per second, and calculate average ourselves.
    //
    match market.get_all_prices() {
        Ok(answer) => {

            match answer {
                // need to match the exact enum
                Prices::AllPrices(data) => {
                    // caching
                    *data_cache = data.clone();
                    
                    for item in data_cache {
                        //
                        // Filter stuffs here
                        // TODO: use HashMap to add also prices
                        let i = item.symbol.as_str();
                        if i.ends_with("USDT") && !symbols_except.contains(&i) { symbols.push(i); }
                        if i.ends_with("BTC") { symbols_BTC.push(i); }
                    }
                }
            }
        },
        Err(e) => println!("Error with data_cache = {:2}\n{:1}", e, &data_cache.len()),
    }

    &symbols.sort();
    for symbol in symbols.clone() { 
        println!("sym {:<16}", &symbol); 
    }
    println!("Total USDT symbols is {}\n", symbols.len());
    //
    // TODO:
    // Merge this filter into above loop, after got all prices,
    // we may later call this a lot instead of each symbol thread 
    // calling its own average_price and current_price.
    //
    for symbol in &symbols {
        let i = &symbol[0..(symbol.len()-4)];
        //
        // excluding leverage symbols & pairs without BTC
        //
        if !&i.contains("UP") && !&i.contains("DOWN") {
            let possible_sym = i.to_owned() + "BTC";
            if symbols_BTC.contains(&possible_sym.as_str()) {
                symmap.insert(&symbol, 0.0);
                println!("i = {}", &symbol);
            }
        }
    }
    println!("Total BTC pairs is {}\n", &symmap.len());

    return symbols;
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

fn whale_scan(symbol: String)
{            
    // fixed stuffs
    let mut epoch = 0;
    let _symbol = symbol.as_str();
    // config file loading
    let mut config = Ini::new();
    let map = config.load("config.toml");
    match map {
        Ok(_) => {
            // let account: Account = Binance::new(api_key.clone(), secret_key.clone());
            let market = get_market(&mut config);
            loop {
                //
                // TODO: Replace with main thread average prices
                //
                match &market.get_average_price(_symbol) {
                    Ok(answer) => { 
                        // increase cycle 
                        epoch += 1; 
                        //
                        // indicate we are riding whale or not 
                        let mut is_whale_riding = false;
                        //
                        // Check Average price first
                        let _average = answer.price;
                        //
                        // compute diff
                        let _diff = compute_change(&market, &_symbol, _average, epoch);
                        //
                        // processing "changes" when we already know it's "Rise" of "Fall"
                        //
                        decision_making(_diff,  &mut is_whale_riding, &_symbol);
                        //
                        // end line
                        println!();
                    },
                    Err(e) => println!("Error: {}", e),
                }
                // sleep for a while
                thread::sleep(two_second);
            }
        },
        Err(_) => println!("{}","can't load config file.".red())
    }
}

fn buy_symbol_with_btc<S>(market: Market, account: Account) 
where S: Into<String> 
{
    println!("Which symbol to buy ? ");

    let mut symbol:String = String::new();
    std::io::stdin()
        .read_line(&mut symbol)
        .ok()
        .expect("invalid symbol !");

    // convert to String to borrow later
    let _symbol:String = symbol.into();

    // Latest price for ONE symbol
    match market.get_price(&_symbol) {
        Ok(answer) => {
            println!("\n- {}: {}", answer.symbol, answer.price);
            let current_price = answer.price;

            // get all BTC 1st 
            match account.get_balance("BTC") {
                Ok(answer) => {
                    println!("- BTC free: {}", answer.free);
                    // "balances": [
                    // {
                    //     "asset": "BTC",
                    //     "free": "4723846.89208129",
                    //     "locked": "0.00000000"
                    // },
                    let available_btc:f64 = answer.free.parse().unwrap();
                    let qty = &available_btc / &current_price;
                    //
                    // we convert all current BTC into the next coin:
                    //

                    println!("- market_buy {} {}", qty ,_symbol);

                    // buy all with btc 
                    match account.market_buy(&_symbol, qty) {
                        Ok(answer) => {
                            println!("- success => {:?}\n", answer)
                        },
                        Err(e) => println!("- ERROR: \n{:?}", e),
                    }
                },
                Err(e) => println!("Error: {:?}", e),
            }
        },
        Err(e) => println!("Error: {:?}", e),
    }

    println!("\n");
}

fn compute_change(market: &Market, symbol: &&str, average:f64, epoch: i32) -> f64
{
    //
    // Compare to latest price 
    //
    let mut _diff = 0.0;
    fn detect_changes(changes: f64) -> String {
        return String::from(if changes > 1.0 {"Gain"} else {"Loss"});
    }
    match market.get_price(*symbol) {
        Ok(answer) => {
            // calculate stuffs 
            let _changes = answer.price / average;
            _diff = (_changes-1.0) * 100.0;
            // log
            let log = format!(
                "[epoch #{}]: {:<10}: average = {:.2} | {}: {:.2}%", 
                epoch, symbol, average, detect_changes(_changes), _diff);
            print!("{}", if _diff > 0.0 {log.green()} else {log.red()});
        },
        Err(e) => println!("Error: {}", e),
    }
    return _diff;  // return
}

fn decision_making(_diff:f64, is_whale_riding: &mut bool, symbol: &&str){
    //
    // Define BUY RATIO here
    //
    let ratio_whale_pump:f64 = 2.0; // percent

    if _diff > ratio_whale_pump {
        //
        // BUY
        //
        print!("{}", format!("=> WHALE DETECTED => BUY XXX {}", symbol).white().bold());
        *is_whale_riding = true;

    } 
    else {
        if *is_whale_riding {
            //
            // SELL
            //
            print!("{}",format!(" => WHALE DUMP => SELL ALL {}", symbol).red());
        } 
        else {
            //
            // DOING NOTHING 
            //
        }
    }
}
