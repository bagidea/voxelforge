# AAA Post-Processing Stack: Golden-Hour Lighting

> Director: Rose  
> Date: 2026-08-06  
> Scope: real, immediately-usable values for the three core post-processing lanes that define a golden-hour cinematic look in a voxel renderer.  
> Constraint: every number below must cite a source; no guessed theory.  
> Cross-ref: `docs/aaa-research-2026-08-06.md` (lighting/fog/PBR lane), `client/src/look.rs`.

---

## Progress log

- `[16:20]` file created, skeleton written
- `[16:21]` sub-agent launched: exposure + tone mapping
- `[16:21]` sub-agent launched: bloom
- `[16:21]` sub-agent launched: volumetric fog / atmosphere

---

## 1. Exposure & Tone Mapping

### 1.1 Parameters to source

| parameter | value | source | note |
|---|---|---|---|
| exposure mode | — | — | — |
| middle-grey | — | — | — |
| min/max exposure | — | — | — |
| tone-mapper | — | — | — |
| white point | — | — | — |

### 1.2 Findings

(TBD)

---

## 2. Bloom

### 2.1 Parameters to source

| parameter | value | source | note |
|---|---|---|---|
| threshold | — | — | — |
| knee | — | — | — |
| intensity | — | — | — |
| radius / spread | — | — | — |
| dirt mask strength | — | — | — |

### 2.2 Findings

(TBD)

---

## 3. Volumetric Fog / Atmosphere

### 3.1 Parameters to source

| parameter | value | source | note |
|---|---|---|---|
| scattering coefficient | — | — | — |
| extinction coefficient | — | — | — |
| phase function / anisotropy | — | — | — |
| density falloff | — | — | — |
| height falloff | — | — | — |

### 3.2 Findings

(TBD)

---

## 4. Executive summary

(TBD)
