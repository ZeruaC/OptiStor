from fastapi import FastAPI

from .api.routes import router as sessions_router
from .api.tariffs import router as tariffs_router

app = FastAPI(title="Balore OptiStor Engine")
app.include_router(sessions_router)
app.include_router(tariffs_router)


@app.get("/health")
def health():
    return {"status": "ok", "service": "optistor-engine"}
