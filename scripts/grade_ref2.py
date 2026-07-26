#!/usr/bin/env python3
"""Refined eyedrop: find true sunlit patch, true teal block, representative deep shade."""
import sys
from PIL import Image
img = Image.open("docs/assets/golden-beauty-shot-ref.png").convert("RGB")
W, H = img.size
px = img.load()
def Lum(r,g,b): return (0.2126*r+0.7152*g+0.0722*b)/255*100
def patch(cx,cy,rad=5):
    rs=gs=bs=n=0
    for y in range(max(0,cy-rad),min(H,cy+rad+1)):
        for x in range(max(0,cx-rad),min(W,cx+rad+1)):
            r,g,b=px[x,y]; rs+=r;gs+=g;bs+=b;n+=1
    return rs/n,gs/n,bs/n

# 1) TRUE brightest sunlit WOOD patch on foreground table (warm, high-L, lower 60% of frame, not window)
print("## brightest warm patches on table/counter (exclude window x<0.22W, y>0.55H)")
cands=[]
for y in range(int(0.55*H),H-6,8):
    for x in range(int(0.22*W),W-6,8):
        r,g,b=patch(x,y,4)
        if r>g>b:  # warm
            cands.append((Lum(r,g,b),x,y,r,g,b))
cands.sort(reverse=True)
for lum,x,y,r,g,b in cands[:5]:
    print(f"  @({x:4d},{y:4d}) RGB=({r:5.1f},{g:5.1f},{b:5.1f}) L={lum:5.1f}% R-B={r-b:+6.1f} R/G/B ratio {r/max(g,1):.2f}/{g/max(b,1):.2f}")

# 2) TRUE teal block: scan for green-dominant pixels (G max channel)
print("\n## teal search: pixels where G>=R and G>=B (green-dominant)")
gx=gy=gn=0; sample=[]
for y in range(0,H,3):
    for x in range(0,W,3):
        r,g,b=px[x,y]
        if g>=r and g>=b and g>25:
            gx+=x;gy+=y;gn+=1
            if len(sample)<3 and g>60: sample.append((x,y,r,g,b))
if gn:
    cx,cy=gx//gn,gy//gn
    print(f"  green-dominant pixel count share = {gn/((H//3)*(W//3))*100:.2f}%  centroid=({cx},{cy})")
    r,g,b=patch(cx,cy,10)
    print(f"  centroid patch RGB=({r:.1f},{g:.1f},{b:.1f}) L={Lum(r,g,b):.1f}%")
    for x,y,r,g,b in sample: print(f"    bright-green sample @({x},{y}) RGB=({r},{g},{b})")
else:
    print("  NONE — no green-dominant pixels at all")

# 3) representative deep under-counter shade: darkest patch EXCLUDING outer 8% border
print("\n## representative deep shade (darkest 11x11, exclude outer 8% border)")
m=int(0.08*W)
best=None
for y in range(m,H-m,10):
    for x in range(m,W-m,10):
        r,g,b=patch(x,y,5)
        lum=Lum(r,g,b)
        if best is None or lum<best[0]: best=(lum,x,y,r,g,b)
lum,x,y,r,g,b=best
print(f"  @({x},{y}) RGB=({r:.1f},{g:.1f},{b:.1f}) L={lum:.2f}% warm(R>=G>=B)={r>=g>=b} R-B={r-b:+.1f}")

# also: distribution of luminance floor — 1st/5th percentile of interior
print("\n## interior luminance percentiles (exclude outer 8%)")
vals=[]
for y in range(m,H-m,3):
    for x in range(m,W-m,3):
        r,g,b=px[x,y]; vals.append(Lum(r,g,b))
vals.sort()
for p in [1,5,10,50,90,99]:
    print(f"  p{p:02d} L = {vals[int(len(vals)*p/100)]:.1f}%")
