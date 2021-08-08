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

  d3.csv("data/dax-2018-2021-daily.csv", map).then((data) => {
    const yExtent = fc
      .extentLinear()
      .pad([0.1, 0.1])
      .accessors([(d) => d.high, (d) => d.low]);

    const xExtent = fc.extentTime().accessors([(d) => d.date]);

    const gridlines = fc.annotationSvgGridline();
    const candlestick = fc
      .autoBandwidth(fc.seriesSvgCandlestick())
      .widthFraction(0.6);

    const zoom = fc.zoom().on("zoom", render); // TODO add zoom extent limiting

    const multi = fc.seriesSvgMulti().series([gridlines, candlestick]);

    const x = fc
      .scaleDiscontinuous(d3.scaleTime()) // FIXME work out how to do this for other chart types
      .discontinuityProvider(fc.discontinuitySkipWeekends())
      .domain(xExtent(data));
    const y = d3.scaleLinear().domain(yExtent(data));

    const chart = fc
      .chartCartesian(x, y)
      .svgPlotArea(multi) // weird?
      .decorate((sel) => {
        sel.enter().selectAll(".plot-area").call(zoom, x, null);
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
