# Alibaba Cloud ECS deployment

This directory provides the reproducible deployment entry point for IndexLink's
Rust backend on Alibaba Cloud ECS. The backend uses the Alibaba Cloud Model
Studio DashScope OpenAI-compatible endpoint through the Qwen client in
[`crates/ai-client/src/provider.rs`](../../crates/ai-client/src/provider.rs).

## Host prerequisites

- Ubuntu 22.04 ECS with Docker Engine, Docker Compose v2, Git and `curl`.
- An inbound security-group rule for TCP `8080`, restricted to the demonstrator's
  current public IP. Keep SSH (`22`) restricted the same way.
- A Git checkout of this repository. Do not install OpenD or provide OpenD
  credentials on the public ECS host: the deployed demo is backend/Qwen only.

## First deployment

```bash
sudo mkdir -p /opt/indexlink
sudo chown "$USER":"$USER" /opt/indexlink
git clone https://github.com/GuZZ1119/indexlink.git /opt/indexlink
cd /opt/indexlink
cp .env.example .env
chmod 600 .env
```

Edit the server-local `.env` with a secret editor. At a minimum set a real
`DASHSCOPE_API_KEY`; keep `APP_HOST=0.0.0.0`, set
`CORS_ALLOWED_ORIGINS` to the deployed frontend origin (or the local Vite
origin for the demo), and leave `OPEND_PROVIDER` unset. Never commit, echo or
copy this file into a container image.

Then deploy and verify:

```bash
./deployment/aliyun/ecs-deploy.sh
curl --fail http://127.0.0.1:8080/ready
```

The Compose named volume keeps the SQLite database across container rebuilds.
To upgrade, pull the reviewed Fork commit and re-run the script:

```bash
git pull --ff-only origin main
./deployment/aliyun/ecs-deploy.sh
```

## Contest proof

Use the following artifacts together:

1. This tracked deployment script, showing a reproducible ECS backend launch.
2. [`crates/ai-client/src/provider.rs`](../../crates/ai-client/src/provider.rs),
   which configures Alibaba Cloud DashScope/Qwen through its documented
   OpenAI-compatible API.
3. An ECS-console screenshot showing the running instance and a terminal
   screenshot of `/ready` returning success. Do not include API keys in either
   screenshot.
