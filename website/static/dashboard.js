const setup = async () => {
  const repository = document.querySelector(
    'script[src="/static/dashboard.js"]'
  ).dataset.repository;
  const res = await fetch(
    `/api/query?repository=${repository}&query=SELECT * FROM build`
  );
  console.log(await res.json());
};

document.addEventListener("DOMContentLoaded", setup);
