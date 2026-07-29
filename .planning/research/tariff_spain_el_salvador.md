# Tariff Regulation Research — Spain & El Salvador

Prepared as domain research input for the Balore OptiStor tariff models (`engine/src/optistor_engine/tariffs/spain.py`, `.../el_salvador.py`, both currently `TariffPending` stubs). **No formula in this document should be implemented as-is** — every component below carries an explicit confidence label, and the "Open questions" lists at the end of each section must be resolved with Benja before coding.

---

## 1. Spain

### Regulatory body and sources

- **CNMC** (Comisión Nacional de los Mercados y la Competencia) sets the *peajes de acceso* (network access tolls) for transport and distribution. Methodology is fixed by **Circular 3/2020** (as modified by **Circular 1/2025**), published in the BOE:
  - [Circular 3/2020, de 15 de enero — metodología peajes de transporte y distribución (BOE-A-2020-1066)](https://www.boe.es/buscar/act.php?id=BOE-A-2020-1066)
  - [Circular 1/2025, de 28 de enero — modifica la Circular 3/2020 (BOE-A-2025-2044)](https://www.boe.es/buscar/doc.php?id=BOE-A-2025-2044)
  - [CNMC — Peajes de transporte y distribución de electricidad 2026 (tramite/resolution page)](https://sede.cnmc.gob.es/tramites/energia-electricidad/peajes-de-transporte-y-distribucion-de-electricidad-2026)
  - [CNMC press release — peajes 2026 aprobados 22/12/2025](https://www.cnmc.es/prensa/peajes-electricidad-2026-20251222)
  - [CNMC — Memoria justificativa peajes 2026 (PDF)](https://www.cnmc.es/sites/default/files/6331870.pdf)
- **Cargos del sistema** (system charges — RE promotion, cogeneration, historical tariff deficit, extra-peninsular costs) are set separately, annually, by **MITECO** (Ministerio para la Transición Ecológica y el Reto Demográfico), not CNMC:
  - [MITECO — Peajes de acceso a las redes... y cargos asociados a los costes del sistema](https://www.miteco.gob.es/en/energia/energia-electrica/electricidad/peajes.html)
- **OMIE** publishes the day-ahead wholesale market price (spot_price), the reference for a free-market ("mercado libre") commercial consumer's energy term — as opposed to PVPC, the regulated residential tariff, which is a *different, non-applicable* product for a commercial/industrial client.
- CNMC consumer-facing explainer on bill structure: [CNMC — "La nueva factura de la luz", preguntas frecuentes (PDF)](https://www.cnmc.es/file/304523/download) — fetched but the tool could not OCR/parse the PDF body; treat as a source to hand-check manually, not as a component this report quotes directly.

### Components of the final price for a market-price (non-PVPC) commercial/industrial consumer

| Component | Confidence | Detail |
|---|---|---|
| **OMIE day-ahead spot price** (`spot_price`) | Confirmed from official source (OMIE is the designated day-ahead market operator for Spain/Portugal) | Base wholesale energy cost. A free-market retailer's energy term is *not required by law* to track OMIE 1:1 — the retail contract can add margin/hedging — but for a proposal-modeling tool, OMIE is the standard proxy. |
| **Peajes de acceso** (network tolls, transport + distribution) | Confirmed from official source | Set annually by CNMC per Circular 3/2020 (as amended by Circular 1/2025). For 2026: average +0.5% vs. 2025, but *industrial* 6.3 TD toll band actually **falls** ~4.1%, while domestic 2.0 TD falls ~1.3% — i.e., 2026 changes are not uniform across consumer classes. [CNMC press release](https://www.cnmc.es/prensa/peajes-electricidad-2026-20251222) |
| **Cargos del sistema** (system charges: RE support, cogeneration, tariff deficit) | Confirmed to exist as a distinct regulated component, set by MITECO not CNMC; exact 2026 value not retrieved | Reported ~+10.5% proposed increase for 2026 per a secondary source (Grupo Alfer); **not verified against an official MITECO resolution** — flag as needs-confirmation for the specific number. |
| **Coeficiente de pérdidas** (loss coefficient) | Confirmed to exist, mechanism only partially confirmed | Per CNMC circulars, standard loss coefficients convert energy measured at the meter to energy at power-plant busbars, used in settlement; CNMC can update them by resolution. **Could not find the exact official formula showing where in the retail price stack this multiplies** (i.e., whether it multiplies spot price only, or the whole pre-tax base). The predecessor prototype's draft applied `losses*(1+municipality)` inside the tax/uplift bracket — **this specific interaction (losses × municipality) was NOT found in any official CNMC/BOE source searched**, and should be treated as unconfirmed/possibly fabricated by the prototype's original author rather than a documented mechanism. |
| **"Municipality" surcharge** | Inferred, low confidence, likely mislabeled | No official "recargo municipal" line item was found in a CNMC/BOE description of a commercial bill. One secondary source (non-official) mentions a ~1.5% municipal tax applied to most bill costs (except peaje) collected by the municipality where the supply point is located, but this was **not corroborated by an official CNMC or Hacienda source** in this pass. There is a distinct, real thing this might be confusing itself with — the **IAE** (Impuesto de Actividades Económicas, a municipal tax that falls on the *supplier/generator*, not typically itemized on the client's bill) — but that's a different mechanism than a per-kWh surcharge. **This needs direct expert/legal confirmation before being encoded as a formula term.** |
| **Impuesto Especial sobre la Electricidad (IEE)** | Confirmed from official/near-official sources, current rate confirmed | Standard rate **5.11% (5.113%)** of the taxable base (energy + power terms + peajes + cargos), applied before IVA. Spain applied a **temporary reduction to 0.5%** between 22 March and 31 May 2026 under an extraordinary royal decree, then **reverted to the standard 5.11% from 1 June 2026**. [Repsol explainer](https://www.repsol.es/particulares/asesoramiento-consumo/impuesto-electricidad-que-es-como-se-paga/), [Agencia Tributaria — medidas extraordinarias IEE, junio 2026](https://sede.agenciatributaria.gob.es/Sede/impuestos-especiales-medioambientales/novedades-impuestos-especiales-medioambientales/2026/junio/30/medidas-extraordinarias-impuesto-especial-sobre-medio.html) |
| **IVA** | Confirmed, current rate confirmed | Standard **21%** VAT, applied on top of energy + power terms + peajes + cargos + alquiler de contador + IEE itself (i.e., IVA is calculated on a base that already includes the electricity tax — tax-on-tax). A temporary reduced 10% IVA existed only for **residential contracted power ≤10 kW**, ended 1 June 2026 — **not applicable to a commercial/industrial consumer**, who should use 21% IVA. [Octopus Energy — bajada IVA luz 2026](https://octopusenergy.es/blog/baja-iva-luz-2026), [IACompara — impuestos luz 2026](https://www.iacompara.es/blog/impuestos-luz-2026-iva-impuesto-electrico) |

### Proposed structure (component order), with confidence

Based on the above, the best-supported *order of operations* (not a finished formula — coefficients/interactions still need confirmation) is:

```
base = spot_price + peajes_acceso + cargos_sistema      [+ possibly a losses multiplier applied somewhere in here — unconfirmed where]
pre_tax_total = base * (1 + coeficiente_perdidas_adjustment)   ← mechanism unconfirmed
con_impuesto = pre_tax_total * (1 + IEE_rate)             ← 1 + 0.0511 under normal conditions
precio_final = con_impuesto * (1 + IVA_rate)              ← 1 + 0.21
```

Confidence: **the tax stacking order (IEE inside, IVA on top of everything including IEE) is confirmed** by multiple consistent secondary sources describing consumer bills; **the losses-coefficient placement is not confirmed** by any official source found in this pass, and the "municipality" term from the inherited prototype could not be traced to any current official mechanism at all. Do not implement the predecessor's `losses*(1+municipality)` term without Benja tracing it to a specific CNMC circular or billing example — it may be a misremembered or invented detail from the original prototype author.

### Open questions for Benja to confirm (Spain)

1. Is the "municipality" term in the old prototype real (a genuine per-kWh municipal surcharge on Spanish electricity bills), or a misunderstanding/artifact from whatever source the original prototype author used? I could not confirm this term against any CNMC/BOE/Hacienda source.
2. Where exactly does the `coeficiente de pérdidas` enter the retail price calculation for a market-price commercial consumer — does it scale the wholesale energy term only, or the full base including peajes/cargos? None of the official CNMC circulars I found state this explicitly for the retail-bill context (they describe pérdidas in the context of settlement/metering, not the client-facing bill formula).
3. Should the model use the **actual current 2026 peaje value** for the client's specific access tariff band (e.g., 6.3 TD for large industrial, 3.0 TD for medium) — CNMC states 6.3 TD tolls are falling ~4.1% in 2026 while the overall average rises 0.5%, so a single "average toll" number would misrepresent an industrial client.
4. Confirm whether the client's contracts are always at 21% IVA (standard commercial) or whether any of Balore's target clients could have contracted power ≤10 kW (the now-expired reduced-IVA band) — almost certainly not relevant for a BESS/PV commercial-scale client, but worth ruling out explicitly.
5. Get the actual numeric 2026 `cargos del sistema` value from an official MITECO order/resolution — the 10.5% figure found here is from a secondary source, not verified against the primary regulatory document.

---

## 2. El Salvador

### Regulatory body and sources

- **SIGET** (Superintendencia General de Electricidad y Telecomunicaciones) regulates and approves distributor tariff sheets ("pliegos tarifarios"). Distributors submit proposed tariffs annually (by the first business day of October) for SIGET approval (by the first business day of December), effective the following January 1; SIGET also issues quarterly bulletins adjusting the energy-charge component.
  - [SIGET — Boletín Pliego Tarifario (quarterly bulletin)](https://www.siget.gob.sv/boletin-pliego-tarifario-1-de-mayo-31-de-julio/)
  - [SIGET — Revisión de Pliegos Tarifarios (process page)](https://www.siget.gob.sv/gerencias/electricidad/servicios-electricidad/electricidad-revision-de-pliegos-tarifarios-presentados-por-las-distribuidoras-de-energia-electrica/)
  - [Reglamento de la Ley General de Electricidad (Decreto Nº 70)](https://www.sc.gob.sv/wp-content/uploads/normativas/Reglamento%20de%20la%20Ley%20General%20de%20Electricidad%20a%20sep18.pdf)
- Distributors (private, AES-owned in the CAESS/CLESA/EEO/DEUSEM footprint) publish their own current tariff-component PDFs:
  - [AES El Salvador — Current fees / tariff category page](https://www.aes-elsalvador.com/en/current-fees)
  - [AES El Salvador — "Informativo para Grandes Clientes" (large-client billing explainer, PDF)](https://aeselsalvador.com/GrandesClientes/web_site/boletines/InfoTF.pdf)
  - Example historical distributor tariff sheet found (not fully parsed numerically in this pass): [CAESS Tarifa 3x6,5 — 15 enero 2025 (PDF)](https://www.aes-elsalvador.com/sites/aesvault.com/files/2025-01/1.CAESS%20Tarifa%203x6,5%2015%20Ene%202025.pdf)

### Wholesale market structure

- **Confirmed**: El Salvador has its own national **Mercado Mayorista**, operated by **UT (Unidad de Transacciones S.A. de C.V.)**, which also operates the transmission system and ensures supply quality. The Mercado Mayorista comprises the **Mercado de Contratos** (bilateral contracts) and the **MRS (Mercado Regulador del Sistema)** — the short-term spot market that balances supply/demand.
  - [UT — Qué hacemos](https://www.ut.com.sv/que-hacemos)
- **Confirmed**: El Salvador additionally participates in the regional **MER (Mercado Eléctrico Regional)**, coordinated by the **EOR (Ente Operador Regional)**, under the **Tratado Marco** (2000) framework, alongside — not replacing — the six national markets (a "seventh market" superimposed on national ones). UT coordinates El Salvador's cross-border MER transactions.
  - [CRIE — MER overview](https://crie.org.gt/mer-2/)
  - [CRIE — El Salvador as MER agent](https://crie.org.gt/mer/agentes-del-mer/el-salvador/)
- **Inferred, needs confirmation**: for a large industrial consumer's *domestic* supply contract in El Salvador, the relevant reference/pass-through price is most likely the **national MRS spot price** (set by UT) rather than the MER cross-border price, since MER transactions are a supplementary/arbitrage layer between national system operators, not the primary domestic settlement price a retail-scale industrial contract would reference. I could not find an explicit SIGET/UT statement confirming this for large-client billing specifically — this is inference from market-structure descriptions, not a directly cited regulatory statement.
- The `.../el_salvador.py` stub's own comment (April 2026 data: El Salvador ~75–146 USD/MWh vs. Nicaragua ~156–178 USD/MWh) is consistent with El Salvador having its own distinct national price rather than a single regional clearing price — supports treating it as its own market entity, as the code comment already concludes.

### Regulated pass-through components (industrial/commercial consumer)

| Component | Confidence | Detail |
|---|---|---|
| **Cargo por energía** (energy charge) | Confirmed from distributor's own site | AES El Salvador's own tariff page states the energy charge is **set quarterly** and is explicitly **"not set by the distribution company"** — i.e., it is a regulated pass-through tied to wholesale/generation cost, consistent with the SIGET quarterly "Boletín Pliego Tarifario" adjustment cycle. |
| **Cargo por Distribución** (distribution charge / VAD-equivalent) | Confirmed to exist as a distinct line item from AES's own tariff page | Covers network operation and expansion — this is the Salvadoran analog of a VAD (valor agregado de distribución) charge, though SIGET/AES materials found in this pass used the label "Cargo por Distribución" rather than "VAD" verbatim. |
| **Cargo por Comercialización** (commercialization/retail charge) | Confirmed to exist | Covers customer service/billing operations (AES describes it as covering 24/7 customer service). |
| **Transmission charge** | Inferred, needs confirmation | UT operates the transmission system and presumably a transmission toll is embedded somewhere in the distributor's approved pliego tarifario, but I could not isolate a distinct "cargo por transmisión" line item separate from distribution in the sources retrieved this pass — may be bundled into distribution/energy charges in the published pliego structure, or may be a separate line not surfaced by the searches run. |
| **Losses** | Inferred, needs confirmation | Standard regulatory practice (as in most Central American markets) is to apply a technical loss factor to the energy charge; SIGET's pliego tarifario approval process almost certainly includes one, but I did not retrieve a document in this pass that states the specific loss factor or its point of application for a given distributor/voltage level. |
| **Demand/capacity charge** | Inferred, needs confirmation | AES's large-client explainer document could not be reliably parsed (PDF binary content) — component was named as generally expected for "Gran Demanda"-type industrial tariffs by analogy with other Central American markets' pliegos (e.g., Panama's ENSA, which does show demand charges in $/kW), not confirmed specifically for El Salvador in this pass. |
| **IVA (VAT)** | Confirmed rate, **application to electricity unresolved/conflicting** | El Salvador's general IVA is **13%**. However, Article 46 of the IVA law (Ley de Impuesto a la Transferencia de Bienes Muebles y a la Prestación de Servicios) exempts electricity, potable water, and sewerage **specifically when supplied by public institutions**. El Salvador's distribution companies in the AES footprint (CAESS, CLESA, EEO, DEUSEM) are **privately owned**, so this public-institution exemption likely does **not** apply to a standard commercial/industrial bill from those distributors — but I found no source stating this conclusion directly; it is my inference from combining the exemption's stated scope with the distributors' private ownership, not a confirmed fact. **This is the single highest-priority item to verify before assuming 13% IVA applies to a commercial client's full electricity bill.** [Ley IVA — Decreto Legislativo](https://transparencia.mh.gob.sv/downloads/pdf/DC5100_LEY_DE_IMPUESTO_A_LA_TRANSFERENCIA_DE_BIENES_MUEBLES_Y_A_LA_PRESTACION_DE_SERVICIOS_-IVA.pdf), [exemptions summary (non-official)](https://ivacalculator.com/el-salvador/exenciones-iva/) |

### Concrete worked example

**Could not find** in this pass: a fully parsed, numeric SIGET-approved pliego tarifario showing the actual $/kWh and $/kW breakdown for a medium/high-voltage industrial consumer. Two promising documents were located but not machine-readable via the fetch tool used (binary PDF content that the fetch/summarization step could not OCR):
- [CAESS Tarifa 3x6,5 — 15 enero 2025 (PDF)](https://www.aes-elsalvador.com/sites/aesvault.com/files/2025-01/1.CAESS%20Tarifa%203x6,5%2015%20Ene%202025.pdf)
- [AES "Informativo para Grandes Clientes" (PDF)](https://aeselsalvador.com/GrandesClientes/web_site/boletines/InfoTF.pdf)

Recommend Benja (or a follow-up research pass with PDF-OCR tooling) open these directly for the actual numeric breakdown — the fetch tool used in this research session repeatedly returned raw/undecoded PDF byte streams instead of parsed text for both SIGET- and AES-hosted PDFs.

### Open questions for Benja to confirm (El Salvador)

1. Does the 13% IVA actually apply to a private distributor's (CAESS/CLESA/EEO/DEUSEM) commercial/industrial electricity bill, given the Article 46 exemption is worded around "public institutions"? This is the single most commercially consequential open question found in this research — a wrong assumption here changes every proposal's tax line by 13%.
2. Is the reference wholesale price for a large industrial consumer's contract the **national MRS spot price (via UT)**, a **bilateral contract price**, or something referencing the **regional MER**? Needs a SIGET/UT source or a real client contract to confirm, not just market-structure descriptions.
3. What is the actual numeric loss factor and its point of application (energy charge only, or full bill) in a current SIGET-approved pliego tarifario?
4. Is there a distinct transmission charge line, or is it bundled into "Cargo por Distribución" in El Salvador's tariff structure? The AES tariff page structure suggests a 3-component split (energy / distribution / commercialization) with no separate transmission line visible — needs confirmation this isn't just an omission in what was retrieved.
5. Get an actual current numeric pliego tarifario (ideally the CAESS or another AES-footprint distributor's medium/high-voltage industrial sheet) parsed for real $/kWh and $/kW figures to validate any formula before it's coded.

---

## Overall confidence summary

- **Spain**: the *shape* of the bill (wholesale price → + peajes (CNMC) → + cargos (MITECO) → × electricity tax → × VAT, with VAT applied last, including on top of the electricity tax) is reasonably well supported by multiple consistent sources. The specific mechanics of `coeficiente de pérdidas` and the "municipality" term from the inherited prototype are **not confirmed** by any official source found — these are the two components most likely to be wrong if implemented from the old formula as-is.
- **El Salvador**: the *3-part distributor charge structure* (energía / distribución / comercialización) and the UT/MER market relationship are reasonably well supported directly from SIGET/UT/AES sources. Numeric values, the loss factor's application point, and — critically — whether the standard 13% IVA actually applies to a private distributor's industrial bill are **not confirmed** and should not be assumed.
