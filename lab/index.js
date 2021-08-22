import init, { run_test } from "./pkg/lab.js";

const lab = async () => {
  await init();

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

  const background = "#023f68";
  const foreground = "#abd4f1";
  const green = "#48c17b";
  const red = "#e4597c";
  const blue = "";

  const priceData = await d3.csv("data/dax-2018-2021-daily.csv", map);
  const { indicators, trades } = run_test(priceData);

  const data = priceData.map((d, i) => ({ ...d, ...indicators[i] }));
  data.trades = trades;

  console.log("Data for charts", data);

  const yExtent = fc
    .extentLinear()
    .pad([0.1, 0.1])
    .accessors([(d) => d.high, (d) => d.low]);

  const xExtent = fc.extentTime().accessors([(d) => d.date]);

  const gridlines = fc.annotationSvgGridline();

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
      sel.enter().attr("stroke", foreground).style("opacity", 0.5)
    );

  // MCDA values on a shifted scale

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
    ]);

  const x = fc
    .scaleDiscontinuous(d3.scaleTime()) // FIXME work out how to do this for other chart types
    .discontinuityProvider(fc.discontinuitySkipWeekends())
    .domain(xExtent(data));

  const y = d3.scaleLinear().domain(yExtent(data));

  const zoom = fc.zoom().on("zoom", render); // TODO add zoom extent limiting
  const chart = fc
    .chartCartesian(x, y)
    .svgPlotArea(multi)
    .decorate((sel) => {
      sel.enter().selectAll(".plot-area").call(zoom, x, null);
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
