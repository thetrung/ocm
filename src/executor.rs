use binance::account::*;

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

