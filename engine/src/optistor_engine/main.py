from fastapi import FastAPI

app = FastAPI(title="Balore OptiStor Engine")


@app.get("/health")
def health():
    return {"status": "ok", "service": "optistor-engine"}
