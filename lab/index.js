import init, { reflect, to_string } from "./pkg/lab.js";

const lab = async () => {
  await init();

  const price = [
    {
      Open: 15320.1,
      Low: 14970.5,
      High: 16220.1,
      Close: 15200.98,
      Volume: 15000,
    },
    {
      Open: 15320.1,
      Low: 14970.5,
      High: 16220.1,
      Close: 15200.98,
      Volume: 15000,
    },
  ];

  const result = reflect(price);
  const text = to_string(price);

  console.log("from betty", result);

  // D3 stuff

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

  d3.csv("data/dax-2018-2021-daily.csv", map).then((data) => {
    const yExtent = fc
      .extentLinear()
      .pad([0.1, 0.1])
      .accessors([(d) => d.high, (d) => d.low]);

    const xExtent = fc.extentTime().accessors([(d) => d.date]);

    const gridlines = fc.annotationSvgGridline();

    const candlestick = fc
      .autoBandwidth(fc.seriesSvgCandlestick())
      .widthFraction(0.6)
      .decorate((sel, datum) => {
        sel
          .enter()
          .style("fill", (d) => (d.close < d.open ? red : green))
          .style("stroke", (d) => (d.close < d.open ? red : green));
      });

    const zoom = fc.zoom().on("zoom", render); // TODO add zoom extent limiting

    const multi = fc.seriesSvgMulti().series([gridlines, candlestick]);

    const x = fc
      .scaleDiscontinuous(d3.scaleTime()) // FIXME work out how to do this for other chart types
      .discontinuityProvider(fc.discontinuitySkipWeekends())
      .domain(xExtent(data));

    const y = d3.scaleLinear().domain(yExtent(data));

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
  });
};

lab();
