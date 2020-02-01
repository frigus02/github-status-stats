# Deployment

## Prerequisites

- Generate secret for Grafana with random passwords.

  ```sh
  kubectl create secret generic ghss-grafana \
      --from-literal username=admin \
      --from-literal password=$(openssl rand -base64 32)
  ```

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

## Deploy new version

A basic deployment works using:

```sh
kustomize build | kubectl apply -f -
```

See [scripts/build.sh](../scripts/build.sh) for detailed steps.
