# Betty, the spread betting bot

A practical exercise in automatic trading. Betty is a set of tools to design and back-test trading strategies and make mechanical trading decisions.

Betty is built for [spread betting](https://en.wikipedia.org/wiki/Spread_betting), and will include integration with _a_ broker offering that form of trading (probably IG).

**PLEASE NOTE**: I don't know what I'm doing. You're free to use Betty, but you'd be doing so at your own risk. Seriously, I have no idea.

## Prior art

Pretty much the entire approach Betty is built around is based on the work of Chris Chillingworth and his [very educational YouTube channel](https://www.youtube.com/user/spreadbetbeginner).

### Why spread betting?

Two basic reasons:

- It's highly leveraged, meaning we don't have to put up the full value of the trade. However, that also means we're risking a lot more than we have! Red flags should go off in your head.
- It's tax free in the UK. How convenient.

So long as we can manage the risk, we could be ok. Setting up a system to make that likely is what Betty is all about.

## Basic concepts

Betty is designed to automate a really simple trading and risk management strategy for, based purely on technical analysis and trend-following. The basic concept of a strategy is to design a system that decides:

- When to enter and exit trades
- Where to place [stop-loss](https://www.investopedia.com/terms/s/stop-lossorder.asp) order
- The size of the trades, given overall capital and risk appetite

Keeping with the idea that there's no real way to predict the market, the trick is in strictly limiting risk, cutting losses early and letting winning trades run.

A strategy is a set of rules which, given a price history of an instrument, produces entry and exit signals. These tell us when to place a trade and when to exit it. To protect against wild swings and simply betting in the wrong direction, the strategy also decides what price to place a stop-loss order, based on the recent price history. That lets us also calculate the trade size and place it.

### Sizing trades based on risk appetite

Given the current price and the stop-loss price, we know how many points difference is at risk if the trade goes wrong. The size of the trade (bet) is given in pounds per point - how many pounds we win/lose for ever one point of difference between entry and exit price. The way Betty decides the size of the trade is based on a maximum risk per trade parameter, expressed as a percentage of our total capital. For example:

Let's say we have a £1000 account and are willing to lose 3% on any single trade. That way we shoul only ever risk amounts we can afford and the amount changes depending on our balance. For our first trade, that will mean we're risking £30. For the sake of this example, we're trading gold with a current price of 1800 points, our strategy just generated a buy signal (we're going "long") and our stop-loss placing rule (based on a recent minimum price) says we should exit from the trade, if the price drops to 1750 points. That's a 50 point difference, we're willing to risk £30 meaning we'd be placing a trade at 0.6 pound per point (which should hopefully be above the broker minimum).

### Signals

To start with, Betty has a pretty simple exponential moving average cross-over strategy built in. Moving averages effectively smooth the price signal to reveal trend. For the strategy we use two of them, one short-term, one long-term. The short-term one will track the price more closely, the long-term one will show a longer-term trend. The signals occur when the short term trend changes direction against, an crosses over the long term one.

In theory it means we're always in the market, either long, or short. In reality it may be helpful to use some extra signal to avoid placing a lot of quick, pointless trades when the market is oscilating around a stable price ("trading sideways").

### Stop-loss

Betty uses a stop-loss placement approach based on [Donchian Channels](https://www.investopedia.com/terms/d/donchianchannels.asp), which is a fancy name for a moving minimum and maximum price for the past number of price frames. The idea of this is that if I bet for the price to go up and it breaks through the recent minimum, I was clearly wrong and should bail.

## Optimising strategies

The fun part of Betty is automatic back testing and optimisation of strategies. You can probably see that the main parameters of the strategy are the lengths of the moving averages and the lengt of the stop loss. There are other constraints influencing the outcome and limiting what we can do (margin requirements, minimum bet size...), but we don't control those.

Therefore, for each general approach to generating signals and setting stop-loss, there is a space of strategies defined by the parameters of the approach (in our case the lengths of the averages and channels). Working out the best strategy works in two stages.

### Back testing a strategy

Given some parameter values, we can back-test the strategy, by "replaying" it on historical data and generating trades it would've placed. This gives us an indication of how well it can perform in terms of various indicators we can calculate. It needs to be said that past performance does NOT guarantee future returns. But it's better than nothing.

_TODO More on measuring performanc when I work that out._

### Optimising the parameters

If we can calculate a performance of a particular strategy, we can also find the set of parameters that makes it perform the best. Either we simply try all the combinations in a sensible range, or we can use some form of heuristic optimisation to make more educated guesses if the primitive approach gets too slow.

### Constraints

The simulation needs to take into account some constraints, such as the spread (~transaction cost), minimum bet size, margin requirements, etc. This is to make sure the strategy results in performance matching the real world with a real broker account.
