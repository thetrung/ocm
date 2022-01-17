# Railgun
A abitrage bot that find and execute profitable trades in symbols loop.

#### USAGE
- Create new file called `config.toml` with content: 

        [keys]
        api_key = "YOUR_BINANCE_API_KEY"
        secret_key = "YOUR_SECRET_KEY"
        
        [configs]
        binance_fees = 0.1
        investment = 5000.0
        warning_ratio = 20.0

        [symbols]
        ignored = BNBBUSD,QTUMBUSD,ICXBUSD,BTSBUSD,NANOBUSD,ONTBUSD,STRATBUSD,AIONBUSD,TOMOBUSD,ERDBUSD,REPBUSD,COMPBUSD,VTHOBUSD,DCRBUSD,IRISBUSD,MKRBUSD,DAIBUSD,ZRXBUSD,BALBUSD,BLZBUSD,JSTBUSD,WNXMBUSD,TRBBUSD,BZRXBUSD,DIABUSD,SWRVBUSD,WINGBUSD,FLMBUSD,UNFIBUSD,USDCBUSD,TUSDBUSD,PAXBUSD,BANDBUSD,OMGBUSD,RLCBUSD,XEMBUSD,LTOBUSD,ADXBUSD,POLYBUSD,RENBUSD,LSKBUSD,HIVEBUSD,STPTBUSD,POWRBUSD,CTXCBUSD,MDTBUSD,NULSBUSD,BIFIBUSD,YFIBUSD
        bridges = BUSD,BNB

- Run

        cargo run

### MODES
- There are 2 modes but I haven't made it into config yet, since I'm still testing both of them to see which one is more advantage.

- Linear Arbitrage : execute one-by-one through the triangle, you only need configured fund amount of stablecoin, it's profitable but slow.
- Parallel Arbitrage : execute Selling first, then 2 others in parallel, it's faster once it get through but slow on BUY order, still profitable but in your chosen bridge pair (etc: BTC-BUSD), so profit may lie in one of them. You need to prepare all coins and add a fixed list of symbol to trade and scan.

### CONCLUSION
- This bot is profitable but very small and slow. The dead part is its speed to get things filled. I don't want to market order everything, since it will take away your little "effort". But, there's still a chance to do so, if we analyze on different orderbook priority to compute profit, just very very rare chance to do so.

### NOTE
- based on binance-rs for API part.
- modify SYM_A_ORDER (B,B) stats & ticker types (in `update_orderbooks` ) will give you different set of profitable trade rings.
- linear and parallel arbitrage need to be well-prepared and change part of code in `analyzer` as commented. 
- using this on your own risk, it's not finished product to use also, in fact, I wrote this to entertain myself while market is red for a week, to experience what a arbitrage bot look like and how effective it could be.

