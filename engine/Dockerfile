FROM python:3.11-slim
WORKDIR /app

# GEKKO bundles a precompiled (Fortran) local solver binary that needs the
# gfortran runtime library — "slim" base images strip this by default.
# UNVERIFIED: no Docker available in the environment this was written in;
# confirm the engine can actually solve (not just import) the first time
# this image is built for real.
RUN apt-get update && apt-get install -y --no-install-recommends libgfortran5 \
    && rm -rf /var/lib/apt/lists/*

COPY pyproject.toml ./
COPY src ./src

RUN pip install --no-cache-dir .

EXPOSE 8001
CMD ["uvicorn", "optistor_engine.main:app", "--app-dir", "src", "--host", "0.0.0.0", "--port", "8001"]
