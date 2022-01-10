use binance::api::*;
use binance::account::*;
use configparser::ini::Ini;


pub fn execute_orderchains(config: &Ini){//, symbol: String, price: f64, qty: f64){

    let account: Account = Binance::new(
        config.get("keys", "api_key"),
        config.get("keys", "secret_key"));

    match account.get_balance("BUSD") {
        Ok(answer) => println!("{:?}", answer),
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


