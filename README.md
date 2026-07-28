# Balore OptiStor

Plataforma web para modelar, dimensionar y optimizar el despacho de sistemas de almacenamiento en baterías (BESS) y generación solar, con dashboard para uso directo por ingenieros.

Autores: B. Ballesteros, M. Ballesteros, Balore Eng.

## Arquitectura

Arquitectura híbrida en dos servicios independientes:

- **`server/`** — Shell de la aplicación web, en Rust (Axum). Gestiona autenticación, proyectos de cliente, la API del dashboard y sirve el frontend. Es lo que corre de cara a los partners.
- **`engine/`** — Microservicio de cálculo, en Python (FastAPI + GEKKO). Resuelve el modelo de optimización de despacho (potencia, energía, degradación, coste). El `server` le delega cada simulación vía HTTP interno.

Esta separación mantiene el motor de optimización (GEKKO, sin equivalente directo en Rust) aislado y sin reescribir, mientras la parte que da la cara al usuario final se beneficia de la robustez de Rust para un servicio multiusuario permanente.

## Desarrollo local

**Motor (Python):**
```bash
cd engine
py -3.11 -m venv .venv
./.venv/Scripts/pip install -e .
./.venv/Scripts/python -m uvicorn optistor_engine.main:app --app-dir src --port 8001
```

**Servidor (Rust):**
```bash
cd server
cargo run
```

El servidor escucha en `http://127.0.0.1:8000`, el motor en `http://127.0.0.1:8001`. `GET /api/engine/health` en el servidor confirma que ambos se comunican correctamente.
