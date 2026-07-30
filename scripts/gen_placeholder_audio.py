#!/usr/bin/env python3
"""Generate CC0 placeholder WAV files for Voxelforge audio system.

Uses only the stdlib (wave + struct + math) — no external dependencies.
Every file is 16-bit mono 44100 Hz PCM.  All generated sounds are original
synthetic waveforms and are hereby dedicated to the public domain (CC0).

Run:  python scripts/gen_placeholder_audio.py
Output: assets/audio/*.wav + assets/audio/CREDITS.md
"""

import math, struct, wave, os, textwrap

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT  = os.path.join(ROOT, "assets", "audio")
SR   = 44100  # sample rate
BITS = 16     # bit depth
AMP  = 0.85   # peak amplitude to avoid clipping on stacked sounds


# -- helpers ------------------------------------------------------------------
def _write_wav(name: str, samples: list[float], sr: int = SR) -> str:
    """Write a mono 16-bit PCM WAV file. `samples` expected in [-1, 1]."""
    path = os.path.join(OUT, f"{name}.wav")
    n = len(samples)
    raw = b""
    for s in samples:
        s = max(-1.0, min(1.0, s))
        raw += struct.pack("<h", int(s * 32767 * AMP))
    with wave.open(path, "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sr)
        wf.writeframes(raw)
    dur = n / sr
    print(f"  {name}.wav  {dur:.2f}s  {len(raw)//1024} KiB")
    return path


def _sine(freq: float, dur: float, sr: int = SR) -> list[float]:
    n = int(dur * sr)
    return [math.sin(2 * math.pi * freq * i / sr) for i in range(n)]


def _noise(dur: float, sr: int = SR) -> list[float]:
    """White noise using a simple LCG (no `random` import to stay pure)."""
    n = int(dur * sr)
    seed = 12345
    out = []
    for _ in range(n):
        seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF
        out.append((seed / 0x7FFFFFFF) * 2.0 - 1.0)
    return out


def _envelope(n: int, attack: float, decay_start: float, release_start: float) -> list[float]:
    """Linear ramp envelope for `n` samples. All times in seconds."""
    a_samp = max(1, int(attack * SR))
    d_samp = int(decay_start * SR) if decay_start > 0 else n
    r_samp = int(release_start * SR) if release_start > 0 else n
    env = []
    for i in range(n):
        if i < a_samp:
            v = i / a_samp
        elif i < d_samp:
            v = 1.0
        elif i < r_samp:
            v = 1.0 - (i - r_samp) / max(1, n - r_samp)
        else:
            v = 1.0 - (i - r_samp) / max(1, n - r_samp)
        env.append(max(0.0, v))
    return env


def _mul(a: list[float], b: list[float]) -> list[float]:
    n = min(len(a), len(b))
    return [a[i] * b[i] for i in range(n)]


def _mix(*signals: list[float]) -> list[float]:
    n = max(len(s) for s in signals)
    out = [0.0] * n
    for s in signals:
        for i, v in enumerate(s):
            out[i] += v
    peak = max(abs(x) for x in out) if out else 1.0
    if peak > 0.0:
        out = [x / peak * AMP for x in out]
    return out


# -- generators ---------------------------------------------------------------
def gen_footstep_grass():
    """Short click with a tiny high-frequency burst — dry grass crunch."""
    dur = 0.08
    n = int(dur * SR)
    click = _noise(dur)
    # band-pass-ish via blending with a high sine
    tone = _sine(2400, dur)
    sig = _mix(click, tone)
    env = _envelope(n, 0.001, 0.02, 0.04)
    _write_wav("footstep_grass", _mul(sig[:n], env))


def gen_footstep_stone():
    """Harder, brighter click — stone impact."""
    dur = 0.06
    n = int(dur * SR)
    click = _noise(dur)
    tone = _sine(3200, dur)
    sig = _mix(click, tone)
    env = _envelope(n, 0.0005, 0.01, 0.03)
    _write_wav("footstep_stone", _mul(sig[:n], env))


def gen_footstep_wood():
    """Lower, hollow knock — wooden plank."""
    dur = 0.10
    n = int(dur * SR)
    tone = _sine(400, dur)
    tone2 = _sine(600, dur * 0.3)
    sig = _mix(tone, tone2, _noise(dur))
    env = _envelope(n, 0.002, 0.03, 0.06)
    _write_wav("footstep_wood", _mul(sig[:n], env))


def gen_footstep_sand():
    """Soft, muffled crunch — sand."""
    dur = 0.12
    n = int(dur * SR)
    sig = _mix(_noise(dur), _sine(200, dur))
    env = _envelope(n, 0.005, 0.04, 0.08)
    _write_wav("footstep_sand", _mul(sig[:n], env))


def gen_swing_light():
    """Fast whoosh — filtered noise sweeping down."""
    dur = 0.30
    n = int(dur * SR)
    noise = _noise(dur)
    # sweep a band-pass centre down
    out = []
    for i in range(n):
        t = i / SR
        freq = 2000 - t * 4000  # sweep down 2 kHz → ~800 Hz
        bw = 300
        carrier = math.sin(2 * math.pi * freq * t)
        mod = math.sin(2 * math.pi * bw * t) * 0.5 + 0.5
        out.append(noise[i] * carrier * mod)
    env = _envelope(n, 0.01, 0.12, 0.22)
    _write_wav("swing_light", _mul(out, env))


def gen_swing_heavy():
    """Slower, weightier whoosh — deeper frequency sweep."""
    dur = 0.45
    n = int(dur * SR)
    noise = _noise(dur)
    out = []
    for i in range(n):
        t = i / SR
        freq = 1200 - t * 2000
        carrier = math.sin(2 * math.pi * freq * t)
        out.append(noise[i] * carrier)
    env = _envelope(n, 0.02, 0.20, 0.35)
    _write_wav("swing_heavy", _mul(out, env))


def gen_hit_light():
    """Short punch impact — low thud + click."""
    dur = 0.15
    n = int(dur * SR)
    thud = _sine(120, dur * 0.5)
    click = _noise(dur)
    sig = _mix(thud, click)
    env = _envelope(n, 0.001, 0.03, 0.08)
    _write_wav("hit_light", _mul(sig[:n], env))


def gen_hit_heavy():
    """Bigger impact — deeper thud + longer noise burst."""
    dur = 0.25
    n = int(dur * SR)
    thud = _sine(80, dur * 0.6)
    thud2 = _sine(160, dur * 0.3)
    noise = _noise(dur)
    sig = _mix(thud, thud2, noise)
    env = _envelope(n, 0.002, 0.04, 0.15)
    _write_wav("hit_heavy", _mul(sig[:n], env))


def gen_hit_block():
    """Metallic clang — high ring with noise impact."""
    dur = 0.30
    n = int(dur * SR)
    # metallic ring: multiple high harmonics
    ring = _sine(1800, dur)
    ring2 = _sine(2400, dur * 0.5)
    ring3 = _sine(3200, dur * 0.3)
    impact = _noise(0.06)
    sig = _mix(ring, ring2, ring3, impact)
    env = _envelope(n, 0.002, 0.06, 0.20)
    _write_wav("hit_block", _mul(sig[:n], env))


def gen_hit_parry():
    """Sharp bright ring — successful parry spark."""
    dur = 0.35
    n = int(dur * SR)
    ring = _sine(2400, dur)
    ring2 = _sine(3600, dur * 0.4)
    ring3 = _sine(4800, dur * 0.2)
    sig = _mix(ring, ring2, ring3)
    env = _envelope(n, 0.003, 0.08, 0.25)
    _write_wav("hit_parry", _mul(sig[:n], env))


def gen_enemy_death():
    """Crumbling/breaking — low noise rumble + crackle."""
    dur = 0.60
    n = int(dur * SR)
    rumble = _sine(60, dur * 0.5)
    crackle = _noise(dur)
    # amplitude-modulate the noise down
    mod = [0.0] * n
    for i in range(n):
        t = i / SR
        mod[i] = 1.0 - (t / dur) ** 0.5
    sig = _mix(rumble, [crackle[i] * mod[i] for i in range(n)])
    env = _envelope(n, 0.005, 0.10, 0.40)
    _write_wav("enemy_death", _mul(sig[:n], env))


def gen_player_hurt():
    """Low grunt-like thud — player taking damage."""
    dur = 0.25
    n = int(dur * SR)
    thud = _sine(100, dur * 0.4)
    thud2 = _sine(200, dur * 0.2)
    noise = _noise(dur * 0.3)
    sig = _mix(thud, thud2, noise)
    env = _envelope(n, 0.002, 0.04, 0.15)
    _write_wav("player_hurt", _mul(sig[:n], env))


def gen_player_death():
    """Heavy final thud + slow fade — death."""
    dur = 1.20
    n = int(dur * SR)
    thud = _sine(60, dur * 0.7)
    thud2 = _sine(100, dur * 0.4)
    rumble = _noise(dur)
    mod = [1.0 - (i / n) ** 0.6 for i in range(n)]
    sig = _mix(thud, thud2, [rumble[i] * mod[i] for i in range(n)])
    env = _envelope(n, 0.005, 0.15, 0.70)
    _write_wav("player_death", _mul(sig[:n], env))


def gen_player_respawn():
    """Ethereal rising chime — respawn."""
    dur = 0.80
    n = int(dur * SR)
    # Rising tone
    out = []
    for i in range(n):
        t = i / SR
        freq = 400 + t * 600  # sweep up
        out.append(math.sin(2 * math.pi * freq * t))
    # Add a sparkle overtone
    sparkle = _sine(1200, dur * 0.4)
    sig = _mix(out, sparkle)
    env = _envelope(n, 0.05, 0.60, 0.70)
    _write_wav("player_respawn", _mul(sig[:n], env))


def gen_ambient_wind():
    """Continuous low wind — filtered noise (looping)."""
    dur = 3.00
    n = int(dur * SR)
    noise = _noise(dur)
    out = []
    for i in range(n):
        t = i / SR
        # slow LFO for gust feel
        lfo = math.sin(2 * math.pi * 0.3 * t) * 0.4 + 0.6
        out.append(noise[i] * lfo * 0.4)
    env = _envelope(n, 0.10, 2.80, 2.80)
    _write_wav("ambient_wind", _mul(out, env))


def gen_ambient_campfire():
    """Crackling fire — noise bursts (looping)."""
    dur = 2.50
    n = int(dur * SR)
    noise = _noise(dur)
    # shape into crackles: short noise bursts at semi-random intervals
    out = [0.0] * n
    crackle_interval = int(SR * 0.06)
    for start in range(0, n, crackle_interval):
        # vary burst presence
        seed = (start * 7919 + 104729) % 17
        if seed < 7:
            continue
        length = int(SR * (0.005 + (seed / 40.0)))
        for j in range(length):
            idx = start + j
            if idx < n:
                out[idx] = noise[idx] * (1.0 - j / length)
    sig = _mix(out, [x * 0.2 for x in _sine(100, dur)])
    env = _envelope(n, 0.05, 2.40, 2.40)
    _write_wav("ambient_campfire", _mul(sig, env))


def gen_ambient_village():
    """Distant murmur — very low filtered noise (looping)."""
    dur = 3.00
    n = int(dur * SR)
    out = _noise(dur)
    # heavily filter: running average acts as low-pass
    filtered = []
    buf = 0.0
    for i in range(n):
        buf = buf * 0.95 + out[i] * 0.05
        filtered.append(buf * 0.5)
    # add very faint tonal elements
    tone = _sine(220, dur)
    sig = _mix(filtered, [t * 0.15 for t in tone])
    env = _envelope(n, 0.20, 2.80, 2.80)
    _write_wav("ambient_village", _mul(sig, env))


# -- main ---------------------------------------------------------------------
def main():
    os.makedirs(OUT, exist_ok=True)
    print(f"Generating placeholder audio into {OUT}/ ...\n")

    # -- footsteps --
    gen_footstep_grass()
    gen_footstep_stone()
    gen_footstep_wood()
    gen_footstep_sand()

    # -- weapon swings --
    gen_swing_light()
    gen_swing_heavy()

    # -- combat feedback --
    gen_hit_light()
    gen_hit_heavy()
    gen_hit_block()
    gen_hit_parry()
    gen_enemy_death()

    # -- player events --
    gen_player_hurt()
    gen_player_death()
    gen_player_respawn()

    # -- ambient (looping) --
    gen_ambient_wind()
    gen_ambient_campfire()
    gen_ambient_village()

    print(f"\nDone — {sum(1 for _ in os.listdir(OUT) if _.endswith('.wav'))} WAV files in {OUT}/")


if __name__ == "__main__":
    main()
