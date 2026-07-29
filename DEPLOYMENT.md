# Despliegue (Fase 6)

**Estado: artefactos listos, sin desplegar todavia.** Este documento cubre lo que existe y lo que
falta para que `Balore OptiStor` corra en un servicio real y alcanzable, no solo en localhost.

## Decision: Fly.io

Elegido sobre una VPS autogestionada porque:
- Soporta volumenes persistentes (necesarios para el fichero SQLite de `server/`).
- Red privada entre apps (`*.internal`) para que `engine/` nunca quede expuesto publicamente —
  solo `server/` le habla, y solo dentro de la red de Fly.
- TLS y proxy incluidos (`force_https`), sin montar nginx/certbot a mano.
- Despliegue por `flyctl deploy`, apropiado para un equipo pequeno que no quiere carga operativa.

Alternativa descartada: VPS + Docker Compose propio — mas control, pero mas trabajo de
mantenimiento (TLS, actualizaciones de seguridad, proxy) para un equipo de dos personas.

## Lo que existe

- `server/Dockerfile` — build multi-stage; la imagen final solo lleva el binario compilado +
  `static/` (las plantillas Askama y las migraciones sqlx se compilan dentro del binario, no hacen
  falta en tiempo de ejecucion).
- `engine/Dockerfile` — Python 3.11 + dependencias del motor, con `libgfortran5` instalado
  explicitamente porque el binario local del solver de GEKKO (compilado en Fortran) probablemente
  lo necesita en una imagen `slim`.
- `server/fly.toml`, `engine/fly.toml` — configuracion por app. `engine/fly.toml` no publica
  ningun puerto publico a proposito.
- `docker-compose.yml` (raiz) — para probar ambos contenedores juntos en local antes de pagar por
  infraestructura real.

## Lo que NO se ha podido verificar

**Docker no esta instalado en el entorno donde se escribio esto.** Ni `docker` ni WSL con una
distribucion estan disponibles, asi que ninguno de los Dockerfiles ni el `docker-compose.yml` se
ha podido construir ni ejecutar de verdad. Estan escritos con cuidado (revisados a mano, siguiendo
las mismas convenciones que ya funcionan en local), pero antes de desplegar a Fly.io de verdad hay
que:

1. Instalar Docker Desktop (o usar una maquina/CI que lo tenga).
2. `docker compose up --build` desde la raiz del repo y confirmar:
   - `curl http://localhost:8000/health` y `.../api/engine/health` responden bien.
   - El motor puede resolver de verdad dentro del contenedor (no solo arrancar) — es el punto de
     mayor riesgo, por el binario de GEKKO mencionado arriba.
3. Solo despues de eso, provisionar las apps reales en Fly.io:
   ```bash
   fly launch --config server/fly.toml --no-deploy   # o el flujo que fly launch proponga
   fly volumes create optistor_data --size 1 --app optistor-server
   fly deploy --config server/fly.toml
   fly deploy --config engine/fly.toml
   ```
   Los nombres de app (`optistor-server`, `optistor-engine`) y la region (`mad`) en los `fly.toml`
   son marcadores de partida, no algo verificado contra una cuenta real — ajustar segun lo que Fly
   proponga al hacer `fly launch`.

## Variables de entorno en produccion

| Variable | Donde | Valor |
|---|---|---|
| `OPTISTOR_DATABASE_URL` | server | `sqlite:///data/optistor.db` (ya en `fly.toml`) |
| `OPTISTOR_ENGINE_URL` | server | `http://optistor-engine.internal:8001` (red privada de Fly) |
| `OPTISTOR_SUPABASE_URL` | server | URL del proyecto Supabase `OptiStor` |
| `OPTISTOR_SUPABASE_PUBLISHABLE_KEY` | server | Clave publica de Supabase (segura de exponer) |
| `OPTISTOR_BIND_ADDR` | server | Por defecto `0.0.0.0:8000`, no hace falta tocarla |

## Pendiente para completar la Fase 6 (DEPLOY-01..03)

- [x] DEPLOY-01: destino de despliegue decidido (Fly.io)
- [ ] DEPLOY-02: `server/` y `engine/` desplegados y alcanzables en una URL real — bloqueado en
  verificar los Dockerfiles con Docker real y despues aprovisionar las apps en Fly.io
- [ ] DEPLOY-03: flujo completo Configurar->Simular->Dashboard verificado en produccion, con una
  cuenta interna y una de partner — bloqueado en DEPLOY-02
