NOTE: 

* 14.01.2022: current situation is, we just need to customize the analysis result
to make a trade better, faster or worse.
only 2 things to adjust here : constant SYM_A,B,C and book tickers order (bid, ask, ask).
with ( 1.0 bid, -2.0 ask, -100.0 ask ) we can get just right amount of pairs, with good gap
distance between both orderbooks.

The worse kind of orderbook is the one with only 1 step in gap, where price move is barely changed.
although, this one is computed to be the most profitable one ( can reach to 6.32% profit ).
