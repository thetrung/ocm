use binance::api::*;
use binance::account::*;
use configparser::ini::Ini;
use std::collections::HashMap;
use crate::exchangeinfo::QuantityInfo;

pub fn execute_final_ring(config: &Ini, 
    final_ring: &Vec<String>, final_ring_prices: &Vec<f64>, 
    optimal_invest: f64, quantity_info: &HashMap<String, QuantityInfo>){
    //
    // Buy > Sell > Sell
    //
    let account: Account = Binance::new(
        config.get("keys", "api_key"),
        config.get("keys", "secret_key"));

    let fees = 1.0 - (0.075 / 100.0);
    let first_buy = final_ring_prices[0].clone().to_string();
    let price_decimal:Vec<&str> = first_buy.split(".").collect();
    let decimal = price_decimal[1].len();
    let before = format!("{:.8}",(optimal_invest/final_ring_prices[0] * fees));
    let buy_qty = before.parse::<f64>().unwrap(); // remove 8 numbers
    println!("> qty: {} => {:?}", buy_qty, decimal);

    return;

    match account.limit_buy(&final_ring[0], buy_qty, final_ring_prices[0]) {
        Ok(answer) => {
            println!("> buy {:?} {}", answer.executed_qty, &final_ring[0]);

            match account.limit_sell(&final_ring[1], answer.executed_qty, final_ring_prices[1]) {
                Ok(answer) => {
                    println!("> sell {:?} {}", answer.executed_qty, &final_ring[1]);

                    match account.limit_sell(&final_ring[2], answer.executed_qty, final_ring_prices[2]) {
                        Ok(answer) => {            
                            println!("> sell {:?} {}", answer.executed_qty, &final_ring[2]);
                        },
                        Err(e) => println!("Error: {:?}", e),
                    }       
                },
                Err(e) => println!("Error: {:?}", e),
            }
        },
        Err(e) => println!("Error: {:?}", e),
    }

    // match account.market_buy("WTCETH", 5) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    match account.get_balance("BUSD") {
        Ok(answer) => println!("> Balance: ${}", answer.free),
        Err(e) => {
            println!("{:?}", e);
            // error::error_handler(e);
        },
    }
    // match account.get_account() {
    //     Ok(answer) => println!("{:?}", answer.balances),
    //     Err(e) => println!("Error: {}", e),
    // }

    // match account.get_account() {
    //     Ok(answer) => println!("{:?}", answer.balances),
    //     Err(e) => println!("{:?}", e),
    // }

    // match account.limit_buy("WTCETH", 10, 0.014000) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.market_buy("WTCETH", 5) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.limit_sell("WTCETH", 10, 0.035000) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.market_sell("WTCETH", 5) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.custom_order("WTCETH", 9999, 0.0123, "SELL", "LIMIT", "IOC") {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }


    // let order_id = 1_957_528;
    // match account.order_status("WTCETH", order_id) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.get_open_orders("WTCETH") {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.cancel_order("WTCETH", order_id) {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

    // match account.cancel_all_open_orders("WTCETH") {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {:?}", e),
    // }

}



//     println!("Which symbol to buy ? ");

//     let mut symbol:String = String::new();
//     std::io::stdin()
//         .read_line(&mut symbol)
//         .ok()
//         .expect("invalid symbol !");


