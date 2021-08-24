import init, { run_test } from "./pkg/lab.js";

const background = "#023f68";
const foreground = "#abd4f1";
const green = "#48c17b";
const red = "#e4597c";
const blue = "";

const getData = async () => {
  const map = (d) => {
    return {
      date: new Date(d.date),
      open: +d.open,
      close: +d.close,
      low: +d.low,
      high: +d.high,
      volume: +d.volume,
    };
  };

  return await d3.csv("data/dax-2018-2021-daily.csv", map);
};

const lab = async () => {
  await init();

  const opts = {
    short: 20, // Short EMA
    long: 50, // Long EMA
    signal: 10, // MACD signal EMA
    entry: 40, // entry threshold
    exit: 40, // exit threshold
    channel: 20, // stop channel length
  };

  const priceData = await getData();
  const { indicators, trades } = run_test(priceData, opts);

  const data = priceData.map((d, i) => ({ ...d, ...indicators[i] }));
  data.trades = trades;

  console.log("Data for charts", data);

  const timeExtent = fc.extentTime().accessors([(d) => d.date]);
  const priceExtent = fc
    .extentLinear()
    .pad([0.4, 0.1])
    .accessors([(d) => d.high, (d) => d.low]);

  // Price chart

  const priceCandles = fc
    .autoBandwidth(fc.seriesSvgCandlestick())
    .widthFraction(0.6)
    .decorate((sel, datum) => {
      sel
        .enter()
        .style("fill", (d) => (d.close < d.open ? red : green))
        .style("stroke", (d) => (d.close < d.open ? red : green));
    });

  // Stop channel

  const lowerStopLine = fc
    .seriesSvgLine()
    .mainValue((d) => d.long_stop)
    .crossValue((d) => d.date)
    .decorate((sel) => {
      sel.enter().attr("stroke", foreground).style("opacity", 0.3);
    });

  const upperStopLine = fc
    .seriesSvgLine()
    .mainValue((d) => d.short_stop)
    .crossValue((d) => d.date)
    .decorate((sel) =>
      sel.enter().attr("stroke", foreground).style("opacity", 0.3)
    );

  const stopArea = fc
    .seriesSvgArea()
    .mainValue((d) => d.short_stop)
    .baseValue((d) => d.long_stop)
    .crossValue((d) => d.date)
    .decorate((sel) =>
      sel.enter().attr("fill", foreground).style("opacity", 0.08)
    );

  // Moving averages

  const shortEMA = fc
    .seriesSvgLine()
    .mainValue((d) => d.short_ema)
    .crossValue((d) => d.date)
    .decorate((sel) => sel.enter().attr("stroke", foreground));

  const longEMA = fc
    .seriesSvgLine()
    .mainValue((d) => d.long_ema)
    .crossValue((d) => d.date)
    .decorate((sel) =>
      sel.enter().attr("stroke", foreground).style("opacity", 0.4)
    );

  // MCDA values on a shifted scale

  const macdExtent = fc
    .extentLinear()
    .accessors([(d) => d.macd])
    .pad([0.1, 4]);
  const macdDomain = macdExtent(data);

  // Maps MACD range to price range to display them together
  // FIXME this is probably limiting in the long run
  // It's likely better to make two chart areas and link their zooming and scrolling together instead
  const macdToPriceScale = d3
    .scaleLinear()
    .domain(macdDomain)
    .range(priceExtent(data));

  const MACD = fc
    .seriesSvgLine()
    .mainValue((d) => macdToPriceScale(d.macd))
    .crossValue((d) => d.date)
    .decorate((sel) => sel.enter().attr("stroke", foreground));

  const MACDSignal = fc
    .seriesSvgLine()
    .mainValue((d) => macdToPriceScale(d.macd_signal))
    .crossValue((d) => d.date)
    .decorate((sel) =>
      sel.enter().attr("stroke", foreground).style("opacity", 0.4)
    );

  const MACDTrend = fc
    .autoBandwidth(fc.seriesSvgBar())
    .mainValue((d) => macdToPriceScale(d.macd_trend))
    .baseValue(() => macdToPriceScale(0))
    .crossValue((d) => d.date)
    .decorate((sel) =>
      sel
        .enter()
        .attr("fill", (d) => {
          if (d.sentiment.indexOf("Bullish") != -1) {
            return green;
          } else if (d.sentiment.indexOf("Bearish") != -1) {
            return red;
          } else {
            return foreground;
          }
        })
        .style("opacity", 0.7)
    );

  // Annotations

  // TODO
  // entry and exit limit bands
  // MACD 0 line

  const gridlines = fc.annotationSvgGridline();

  const multi = fc
    .seriesSvgMulti()
    .series([
      gridlines,
      lowerStopLine,
      upperStopLine,
      stopArea,
      priceCandles,
      shortEMA,
      longEMA,
      MACD,
      MACDSignal,
      MACDTrend,
    ]);

  const xScale = fc
    .scaleDiscontinuous(d3.scaleTime())
    .discontinuityProvider(fc.discontinuitySkipWeekends())
    .domain(timeExtent(data));

  const yScale = d3.scaleLinear().domain(priceExtent(data));

  // TODO add MACD scale on the left
  // TODO add scope

  const zoom = fc.zoom().on("zoom", render); // TODO add zoom extent limiting
  const chart = fc
    .chartCartesian(xScale, yScale)
    .svgPlotArea(multi)
    .decorate((sel) => {
      sel.enter().selectAll(".plot-area").call(zoom, xScale, null);
    })
    .xDecorate((sel) => {
      sel.enter().selectAll("text").attr("fill", foreground);
      sel.enter().selectAll("path").attr("stroke", foreground);
    })
    .yDecorate((sel) => {
      sel.enter().selectAll("text").attr("fill", foreground);
      sel.enter().selectAll("path").attr("stroke", foreground);
    });

  // Drawing function, to update the chart
  function render() {
    d3.select("#chart").datum(data).call(chart);
  }

  // first render
  render();
};

lab();
