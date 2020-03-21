# Deployment

## Prerequisites

- Generate secret for InfluxDB with random passwords.

  ```sh
  kubectl create secret generic ghss-influxdb \
      --from-literal admin_username=admin \
      --from-literal admin_password=$(openssl rand -base64 32) \
      --from-literal read_password=$(openssl rand -base64 32)
  ```

- Create secret for communication with GitHub.

  1. Go to your GitHub app's settings page (e.g. https://github.com/settings/apps/status-stats).
  1. Copy values for client ID and client secret from the _About_ section.
  1. Generate and enter a random _Webhook secret_ (e.g. `openssl rand -base64 32`).
  1. Create a private key and download the PEM file.

  ```sh
  kubectl create secret generic ghss-github \
      --from-literal CLIENT_ID=<client id> \
      --from-literal CLIENT_SECRET=<client secret> \
      --from-literal WEBHOOK_SECRET=<webhook secret> \
      --from-file PRIVATE_KEY=<path to pem file>
  ```

- Create secret for website.

  ```sh
  kubectl create secret generic ghss-website \
      --from-literal TOKEN_SECRET=$(openssl rand -hex 20)
  ```

- Create secret for Honeycomb.io.

  ```sh
  kubectl create secret generic ghss-honeycomb \
      --from-literal API_KEY=<api key> \
      --from-literal DATASET=<dataset name>
  ```

## Deploy new version

A basic deployment works using:

```sh
kustomize build | kubectl apply -f -
```

See [scripts/build.sh](../scripts/build.sh) for detailed steps.

## Local development on Docker Desktop or Docker for Mac

- [Deploy NGINX Ingress controller](https://kubernetes.github.io/ingress-nginx/deploy/).

  ```sh
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/nginx-0.29.0/deploy/static/mandatory.yaml
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/nginx-0.29.0/deploy/static/provider/cloud-generic.yaml
  ```

- Create localhost ingress for website.

  ```sh
  kubectl apply -f crates/ghss_website/ingress-localhost.yml
  ```
