# Rinha de Backend 2026 - API de Score de Fraude

API em Rust para cálculo de risco de fraude via busca vetorial no Qdrant.

## Arquitetura

O projeto segue uma separação em camadas (DDD pragmático), organizada em `src/`:

- `app/`: bootstrap da aplicação (`run`), leitura de config, inicialização de cliente HTTP/Qdrant e servidor Axum.
- `interfaces/http/`: borda HTTP (`/ready` e `/fraud-score`), roteamento e handlers.
- `application/`: serviços de caso de uso:
  - `fraud_score_service`: vetoriza transação e consulta vizinhos no Qdrant.
  - `readiness_service`: health/readiness de startup e operação.
  - `ingestion_service`: ingestão do dataset de referência no Qdrant.
- `domain/`: modelos de entrada/saída e lógica de vetorização (`VECTOR_SIZE = 14`).
- `infrastructure/`: integração com Qdrant, parser de config por variáveis de ambiente e loaders de recursos.
- `shared/`: tipos de erro e mapeamento para respostas HTTP.

### Fluxo de score

1. `POST /fraud-score` recebe a transação.
2. A transação é transformada em vetor de 14 features normalizadas.
3. A API busca 5 vizinhos no Qdrant.
4. `fraud_score = fraud_neighbors / 5`.
5. `approved = fraud_score < 0.6`.

## Stack escolhida

- **Rust 2024**
- **Axum + Tokio** (API HTTP assíncrona)
- **Reqwest** (cliente HTTP para Qdrant)
- **Qdrant** (busca vetorial)
- **Serde / Serde JSON / Chrono / Flate2**
- **Docker** para build/execução
- **Nginx + Docker Compose** na branch `submission` (ambiente da entrega)

## Como rodar (apenas Docker)

O projeto é executado pela branch `submission`, sem fluxo manual de build/rede.

```bash
git checkout submission
docker compose up --build -d
```

Para acompanhar os logs:

```bash
docker compose logs -f
```

## Entrega da Rinha

- `main`: código-fonte da API.
- `submission`: arquivos mínimos para execução do teste (`docker-compose.yml`, `nginx.conf`, `info.json` e recursos).
