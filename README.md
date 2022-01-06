### OCM - Orders Chain Maker
A binance bot that find profitable chance by chaining orders loop.
* based on whale_hunter sketch github.

##### Usage
1. Config
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

2. Run

        cargo run
