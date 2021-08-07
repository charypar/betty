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

  const data = fc.randomFinancial()(50);

  const yExtent = fc.extentLinear().accessors([(d) => d.high, (d) => d.low]);

  const xExtent = fc.extentDate().accessors([(d) => d.date]);

  const gridlines = fc.annotationSvgGridline();
  const candlestick = fc.seriesSvgCandlestick();
  const multi = fc.seriesSvgMulti().series([gridlines, candlestick]);

  const chart = fc
    .chartCartesian(d3.scaleTime(), d3.scaleLinear())
    .svgPlotArea(multi);

  chart.xDomain(xExtent(data));
  chart.yDomain(yExtent(data));

  d3.select("#chart").datum(data).call(chart);
};

lab();
