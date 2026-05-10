# Rinha de Backend 2026 - API de Score de Fraude

API em Rust para cálculo de risco de fraude com **índice vetorial local** pré-processado no build.

## Arquitetura

- `app/`: bootstrap da aplicação.
- `interfaces/http/`: `GET /ready` e `POST /fraud-score`.
- `application/`: scoring e readiness.
- `domain/`: modelos e vetorização de 14 dimensões.
- `infrastructure/`: config, carregamento dos recursos e índice local.
- `shared/`: erros e respostas HTTP.

### Fluxo de score

1. `POST /fraud-score` recebe a transação.
2. A transação vira um vetor de 14 dimensões.
3. A API consulta um índice local compacto.
4. `fraud_score = fraud_neighbors / 5`.
5. `approved = fraud_score < 0.6`.

## Build e execução

O Dockerfile gera `resources/index.bin` a partir de `resources/references.json.gz` durante o build da imagem.

```bash
docker build -t joaomiraya/rinha-backend-2026:1.0.3 .
docker compose up -d
```

O `docker-compose.yml` usa duas instâncias da API com balanceamento em `nginx` na porta `9999`.

## Stack

- Rust 2024
- Axum + Tokio
- Serde / Serde JSON / Chrono / Flate2
- Docker + Nginx

## Entrega

- `main`: código-fonte e Dockerfile.
- `submission`: `docker-compose.yml`, `nginx.conf`, `info.json` e recursos mínimos para execução.
