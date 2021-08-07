import init, { greet } from "./pkg/lab.js";

const lab = async () => {
  await init();

  const result = greet("Viktor");

  document.getElementById("output").textContent = `Betty says: ${result}`;
};

lab();
