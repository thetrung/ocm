use std::{ 
    collections::HashMap
};

use serde::{Deserialize, Serialize};
use serde_json::Result; 

#[derive(Serialize, Deserialize)]
struct ExchangeInfo {
    timezone: String,
    serverTime: i64,
    rateLimits: Vec<RateLimits>,
    exchangeFilters: Vec<String>,
    symbols:	Vec<Symbol>
}

#[derive(Serialize, Deserialize)]
struct RateLimits {
    rateLimitType : String,
    interval : String,
    intervalNum	: i64,
    limit : i64
}

#[derive(Serialize, Deserialize)]
struct Filter {
    filterType: String,       // LOT_SIZE : since MARKET_LOT_SIZE = "0.00000000" most of time.
    minQty: Option<String>,   // min qty -> qty cap
    maxQty: Option<String>,   // max qty -> qty cap
    stepSize: Option<String>, // decimal -> qty cap
    tickSize: Option<String>, // price lot -> "0.00000100"
    // 
    // not important stuffs
    //
    minPrice: Option<String>, // fetched from tickers
    maxPrice: Option<String>, // fetched from tickers
    multiplierUp: Option<String>, 
    multiplierDown: Option<String>,
    avgPriceMins	:Option<i32>,
    minNotional: Option<String>,
    applyToMarket	: Option<bool>,
    limit	:Option<i32>, // iceberg parts
    maxNumOrders	: Option<i32>,
    maxNumAlgoOrders	:Option<i32>,
}

#[derive(Serialize, Deserialize)]
struct Symbol {
    symbol	: String,       // pair symbol
    status		: String,   // trading or not ?
    filters	: Vec<Filter>,  // <----- we need this shit 
    //
    // not important stuffs
    //
    baseAsset		: String,
    baseAssetPrecision	: i32,
    quoteAsset: String,
    quotePrecision: i32,
    quoteAssetPrecision: i32,
    baseCommissionPrecision: i32,
    quoteCommissionPrecision: i32,
    orderTypes: Vec<String>,
    icebergAllowed : bool,
    ocoAllowed : bool,
    quoteOrderQtyMarketAllowed : bool,
    isSpotTradingAllowed : bool,
    isMarginTradingAllowed : bool,
    permissions	: Vec<String>
}

#[derive(Serialize, Deserialize)]
pub struct QuantityInfo {
    pub symbol	: String,       // pair symbol
    pub min_qty: String,        // min qty -> qty cap
    pub max_qty: String,        // max qty -> qty cap
    pub step_qty: f64,          // step size
    pub step_price: f64,        // step price
    pub move_qty: f64,          // we pre-calculate for step_qty correction 
    pub move_price: f64         // we pre-calculate for step_price correction 
}

impl Default for QuantityInfo { 
    fn default() -> QuantityInfo {
        QuantityInfo { 
            symbol: String::new(), 
            min_qty: String::new(), 
            max_qty: String::new(), 
            step_qty: 0.0,
            step_price: 0.0,
            move_qty: 0.0,
            move_price: 0.0
        }
    }
}

const QUANTITY_INFO_FILE:&str = "quantity.cache";

/// fetch and build exchange info map
pub fn fetch(symbols_cache: &Vec<String>) -> Option<HashMap<String, QuantityInfo>>{
    //
    // 1. Fetch + Map Data into Structs
    let mut quantity_info: HashMap<String, QuantityInfo> = HashMap::new();

    let mut cache_file = configparser::ini::Ini::new();
    match cache_file.load(QUANTITY_INFO_FILE) {
        Ok(_) => { 
            for sym_map in cache_file.get_map() {
                for _quantity_info in sym_map {
                    for stuff in _quantity_info.1 {
                        quantity_info = serde_json::from_str(stuff.1.unwrap().as_str()).unwrap();
                        // println!("{:?}", stuff.1);
                    }
                }
            }
            println!("> loaded quantity info.");
            return Some(quantity_info);
        },
        _error => {
            let req = reqwest::blocking::get("https://www.binance.com/api/v3/exchangeInfo");
            match req {
                Ok(res) => {
                    let content = res.text();
                    // println!("{:?}", content);
                    let exchange_info:ExchangeInfo = serde_json::from_str(&content.unwrap()).unwrap();
                    // start building map
                    for symbol in &exchange_info.symbols {
                        if symbols_cache.contains(&symbol.symbol) {
                            //
                            // init data
                            let _symbol = String::from(&symbol.symbol);
                            let mut _min_qty = String::new();
                            let mut _max_qty =String::new();
                            let mut _step_qty: f64 = 0.0;
                            let mut _step_price : f64 = 0.0;
                            let mut _move_qty : f64 = 0.0;
                            let mut _move_price : f64 = 0.0;
                            //
                            // collect quantity info for 1 symbol :
                            for filter in &symbol.filters {
                                match &filter.filterType.as_str() { 
                                    &"LOT_SIZE" => {
                                        let _step_size = filter.stepSize.as_ref().unwrap();
                                        let _step_size_decimal = move_decimal(_step_size);
                                        _min_qty = filter.minQty.as_ref().unwrap().clone(); 
                                        _max_qty = filter.maxQty.as_ref().unwrap().clone(); 
                                        _step_qty = _step_size.parse::<f64>().unwrap();
                                        _move_qty = _step_size_decimal;
                                    },
                                    &"PRICE_FILTER" => {
                                        let tick_size = filter.tickSize.as_ref().unwrap();
                                        _step_price = tick_size.parse::<f64>().unwrap();
                                        _move_price = move_decimal(tick_size);
                                    }
                                    _ => {}
                                }
                                let new_quantity_info = QuantityInfo {
                                    symbol : _symbol.clone(),
                                    min_qty : _min_qty.clone(), 
                                    max_qty : _max_qty.clone(), 
                                    step_qty : _step_qty,
                                    step_price : _step_price,
                                    move_qty : _move_qty,
                                    move_price: _move_price
                                };
                                quantity_info.insert(_symbol.clone(), new_quantity_info);
                            }
                        }
                    }
                    // Save all 
                    cache_file.set("quantity_info", "json" , Some(serde_json::to_string(&quantity_info).unwrap()));
                    match cache_file.write(QUANTITY_INFO_FILE) {
                        Ok(_) => println!("> saved quantity info to cache."),
                        msg => println!("Error saving cache: {:?}", msg)
                    }
                    return Some(quantity_info);
                }
                _ => return None
            }
        }
    }
}

fn move_decimal (step: &str) -> f64 {
    let arr:Vec<&str> = step.split(".").collect();
    let decimal_arr:Vec<&str> = arr[1].split("1").collect();
    let decimal_point = match decimal_arr.len() {
        2 => { // less than 1.00..
            (decimal_arr[0].len() + 1) as f64 
        },
        1 => { // is 1.000
            0.0 // no decimal point. 
        },
        _ => {println!("ERROR ?"); 0.0}
    };
    let one_decimal:f64 = 10.0;
    let move_qty = one_decimal.powf(decimal_point);
    // result
    return move_qty
}