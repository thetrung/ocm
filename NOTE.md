NOTE: 

* 14.01.2022: current situation is, we just need to customize the analysis result
to make a trade better, faster or worse.
only 2 things to adjust here : constant SYM_A,B,C and book tickers order (bid, ask, ask).
with ( 2.0 bid, -2.0 ask, -100.0 ask ) we can get just right amount of pairs, with good gap
distance between both orderbooks.

The worse kind of orderbook is the one with only 1 step in gap, where price move is barely changed.
although, this one is computed to be the most profitable one ( can reach to 6.32% profit ).

* 20.01.2022 :
1 - thing I know while toying with arbitrage bot is, like Uniswap trade route, this type of bot will try to snipe the best route but with negative trading fees. Most of time, like 99.9%, most triangles on binance are optimized to the point that bots are no longer profitable.

2 - ask-bid spreads on each pair is a chance to make arbitrage work, but narrow gap of it make be really hard, even when volume is high or low.

3 - What kill arbitrage isn't that it doesn't make profit, but time to finish each trade. I mean, to earn 0.5% per trade is still possible, but time to make every order filled would kill the effectiveness of it.

4 - parallel execution may help with order filling time, but you get risk about IL like with Liquidity Provider, since you need as many type of assets as you are trading on. And half-filling order may still make any trade stuck at one point, and you still need to deal with it.

Summary ::
In overall, this is fun to understand about how triangle arbitrage work, as a narrow field to thrive in trading, beside all AI and algorithmic stuffs. But from here, we can see a more broader view on possibilities in crypto market.