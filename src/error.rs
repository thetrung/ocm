use binance::errors::ErrorKind as BinanceLibErrorKind;

// pub fn error_handler<T,E>(e: Result<T,E>){
//     match e {
//         BinanceLibErrorKind::BinanceError(response) => match response.code {
//             -1000_i16 => println!("An unknown error occured while processing the request. {}", response.msg),
//             -1001_i16 => println!("Funds insufficient! {}", response.msg),
//             -1002_i16 => println!("Funds insufficient! {}", response.msg),
//             -1003_i16 => println!("TOO_MANY_REQUESTS {}", response.msg),
//             -1002_i16 => println!("Funds insufficient! {}", response.msg),
//             -2010_i16 => println!("Funds insufficient! {}", response.msg),
//             _ => println!("Non-catched code {}: {}", response.code, response.msg)

//         }
//     }
// }
            // -1000 UNKNOWN
    
            //     An unknown error occured while processing the request.
    
            // -1001 DISCONNECTED
    
            //     Internal error; unable to process your request. Please try again.
    
            // -1002 UNAUTHORIZED
    
            //     You are not authorized to execute this request.
    
            // -1003 TOO_MANY_REQUESTS
    
            //     Too many requests queued.
            //     Too much request weight used; please use the websocket for live updates to avoid polling the API.
            //     Too much request weight used; current limit is %s request weight per %s %s. Please use the websocket for live updates to avoid polling the API.
            //     Way too much request weight used; IP banned until %s. Please use the websocket for live updates to avoid bans.
    
            // -1006 UNEXPECTED_RESP
    
            //     An unexpected response was received from the message bus. Execution status unknown.
    
            // -1007 TIMEOUT
    
            //     Timeout waiting for response from backend server. Send status unknown; execution status unknown.
    
            // -1014 UNKNOWN_ORDER_COMPOSITION
    
            //     Unsupported order combination.
    
            // -1015 TOO_MANY_ORDERS
    
            //     Too many new orders.
            //     Too many new orders; current limit is %s orders per %s.
    
            // -1016 SERVICE_SHUTTING_DOWN
    
            //     This service is no longer available.
    
            // -1020 UNSUPPORTED_OPERATION
    
            //     This operation is not supported.
    
            // -1021 INVALID_TIMESTAMP
    
            //     Timestamp for this request is outside of the recvWindow.
            //     Timestamp for this request was 1000ms ahead of the server's time.
    
            // -1022 INVALID_SIGNATURE
    
            //     Signature for this request is not valid.
