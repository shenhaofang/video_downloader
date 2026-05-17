import "./styles.css";

const root = document.querySelector<HTMLDivElement>("#app");
if (!root) {
  throw new Error("Missing #app root");
}

root.innerHTML = `<main class="boot-screen"><h1>Video Downloader</h1><p>应用初始化中</p></main>`;
