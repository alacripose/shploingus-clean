Aerosol Optics, Sky Model Validity, Cloud LWC, and Twilight Illuminance: A Technical Reference
1. Aerosol Optical Properties and Visibility
1.1 Ångström Turbidity Law and Aerosol Optical Depth
The spectral dependence of aerosol optical depth (AOD) is described by the Ångström power law:

τ
a
(
λ
)
=
β
 
λ
−
α
τ 
a
​
 (λ)=βλ 
−α
 
where 
β
β is the Ångström turbidity coefficient (dimensionless, representing aerosol column loading) and 
α
α is the Ångström exponent (wavelength exponent), which characterizes the aerosol size distribution. Typical values for 
β
β range from 0 to 0.5, while 
α
α typically ranges from 0.5 to 2.5, with 1.3 being a common default. Higher 
α
α values indicate dominance of fine-mode particles (e.g., anthropogenic emissions, biomass burning), while lower values indicate coarse-mode dominance (e.g., dust).

The Ångström exponent is computed from AOD measurements at two wavelengths:

α
=
−
ln
⁡
(
τ
λ
1
/
τ
λ
2
)
ln
⁡
(
λ
1
/
λ
2
)
α=− 
ln(λ 
1
​
 /λ 
2
​
 )
ln(τ 
λ 
1
​
 
​
 /τ 
λ 
2
​
 
​
 )
​
 
This relationship allows extrapolation of AOD from one wavelength to another. The Ångström exponent is inversely related to mean particle size, assuming the power law is a valid representation of optical depth.

1.2 Visibility–Turbidity Relationship
Atmospheric visibility is fundamentally controlled by aerosol concentration. An inverse relationship exists between turbidity and visibility: increases in aerosol turbidity lead to reduced visibility due to enhanced scattering and absorption of solar radiation. This inverse coupling is a robust observational feature across diverse environments.

The Koschmieder relation links visibility 
V
V to the extinction coefficient:

V
=
3.912
b
ext
V= 
b 
ext
​
 
3.912
​
 
where 
b
ext
b 
ext
​
  is the total extinction coefficient (sum of scattering and absorption). The turbidity index of Linke (1942) is defined proportional to the total scattering coefficient at 0.55 μm, establishing a linear relationship between backscattering and turbidity.

Empirical studies have established quantitative relations between visibility, nephelometer scattering coefficient 
b
b, and turbidity coefficient 
B
(
λ
)
B(λ). For example, 
V
=
A
b
−
1
V=Ab 
−1
  with 
A
A ranging between 1.5 and 2.3, and 
V
=
1.7
b
−
1
V=1.7b 
−1
  over a multi-year period, with correlation coefficients approaching unity for visibility–scattering relationships.

1.3 Preetham Turbidity Parameter
The turbidity parameter 
T
T in the Preetham model is the ratio of the optical thickness of the atmosphere including haze to that of a pure-air molecular atmosphere:

T
=
τ
m
+
τ
h
τ
m
=
a
Air
+
a
Aerosol
a
Air
T= 
τ 
m
​
 
τ 
m
​
 +τ 
h
​
 
​
 = 
a 
Air
​
 
a 
Air
​
 +a 
Aerosol
​
 
​
 
A perfectly clean atmosphere has 
T
=
1
T=1. Values around 1.25 are "exceptionally clear," 1.6 is "very clear," and haze begins at 
T
≳
10
T≳10. The model summarizes the influence of water vapour and aerosol pollution into this single parameter.

1.4 Coupling to Wet Deposition (WET_TURBIDITY_GAIN, WATER_PATH_REFERENCE_M)
The coupling between cloud liquid water path and turbidity is physically grounded in wet scavenging and hygroscopic growth. Precipitable water (PW) and turbidity coefficient 
β
β show an inverse coupling: high precipitation efficiently removes aerosols, decreasing 
β
β and increasing visibility. During monsoon periods, PW can reach 4.5–5.0 cm while 
β
β drops to 0.05–0.10, demonstrating strong wet scavenging.

The stylized parameters WET_TURBIDITY_GAIN and WATER_PATH_REFERENCE_M can be sourced from the empirical relation:

Vis
=
0.0219
×
LWC
−
0.9603
Vis=0.0219×LWC 
−0.9603
 
which links visibility directly to liquid water content, providing a quantitative basis for the ramp.

1.5 Summary Table: Aerosol Optical Parameters
Parameter	Symbol	Typical Range	Physical Meaning
Ångström turbidity coefficient	
β
β	0–0.5	Aerosol column loading
Ångström exponent	
α
α	0.5–2.5 (default 1.3)	Particle size (high = fine mode)
Preetham turbidity	
T
T	1–10+	Ratio of total to molecular optical thickness
Visibility	
V
V	0.1–100+ km	Distance of contrast threshold
Extinction coefficient	
b
ext
b 
ext
​
 	0.001–10 km⁻¹	Total scattering + absorption
2. Sky Radiance Models: Validity Domain and Cloud Treatment
2.1 Preetham Model: Clear-Sky Only
The Preetham et al. (1999) analytic sky model is fitted to simulation data from the Nishita et al. (1996) model, which in turn was fitted to the Pérez et al. (1993) luminance approximation. The simulation and fitting only took turbidities in the range 
T
=
2
…
6
T=2…6 into account.

2.2 Known Limitations
Zotti, Wilkie & Purgathofer (2007) performed the first systematic verification against CIE 2003 Standard General Sky and real-world measurements. Their findings:

Low turbidity failure: For 
T
<
2
T<2, the model predicts an unrealistic "horizon glow" — a wide, extremely bright zone that does not occur in nature.

High turbidity failure: Even for 
T
>
2.4
T>2.4, the horizon is unnaturally bright when the sun is low, and the brightening around the sun is too weak.

Antisolar hemisphere: The model does not properly reproduce the darkening of the sky in the antisolar hemisphere; it predicts luminance 2–5× too high.

Negative zenith luminance: The model yields negative values near boundaries of low zenith angle / low turbidity and high zenith angle / high turbidity.

Valid turbidity range: The model should not be used for 
T
≲
1.6
T≲1.6 with the sun near zenith, nor for 
T
≳
10
T≳10 (haze regime).

2.3 Clouds Are Not Aerosol
The Preetham turbidity parameter 
T
T summarizes aerosol and water vapour influence on clear-sky scattering. Clouds are a fundamentally different optical regime: they are composed of liquid water droplets with sizes comparable to or larger than visible wavelengths (Mie scattering regime), and their optical properties are governed by liquid water path and droplet effective radius, not by the molecular/haze turbidity ratio. Clouds should not be treated as an increase in 
T
T; doing so conflates Mie scattering by cloud droplets with Mie scattering by aerosol particles, which have different phase functions, single-scattering albedos, and spatial distributions.

2.4 Recommended Approach for Cloudy Skies
For scenes involving clouds, the Preetham model should be used only for the clear-sky component, with cloud radiance handled by a separate layer. The CIE 2003 Standard General Sky provides models for overcast and intermediate skies, though these lack spectral and colour information. A physically-based approach is to compute clear-sky radiance from Preetham and add cloud radiance using a separate cloud radiative transfer model or a parameterized cloud luminance distribution.

3. Cloud Liquid Water Content Profiles: Adiabatic Model and LWP
3.1 Adiabatic LWC Profile
In a perfect adiabatic cloud, the liquid water content (LWC) increases linearly with height above cloud base:

LWC
(
z
)
=
Γ
ad
(
T
b
,
p
b
)
⋅
(
z
−
z
cb
)
+
LWC
0
LWC(z)=Γ 
ad
​
 (T 
b
​
 ,p 
b
​
 )⋅(z−z 
cb
​
 )+LWC 
0
​
 
where 
Γ
ad
Γ 
ad
​
  is the adiabatic condensation rate (negative of the change in saturation mixing ratio with height), 
z
cb
z 
cb
​
  is cloud base height, and 
LWC
0
LWC 
0
​
  is the liquid water content at cloud base (zero for a perfect adiabatic cloud).

The adiabatic condensation rate 
Γ
ad
Γ 
ad
​
  is evaluated at cloud base using cloud-base temperature 
T
b
T 
b
​
  (K) and pressure 
p
b
p 
b
​
  (Pa). This linear profile is the first assumption examined in adiabatic cloud studies and is widely used in remote sensing retrievals.

3.2 Liquid Water Path (LWP)
The liquid water path is obtained by integrating LWC from cloud base to cloud top:

LWP
=
∫
z
cb
z
ct
LWC
(
z
)
 
d
z
LWP=∫ 
z 
cb
​
 
z 
ct
​
 
​
 LWC(z)dz
For a perfect adiabatic cloud with 
LWC
0
=
0
LWC 
0
​
 =0, this yields:

LWP
=
1
2
Γ
ad
(
T
b
,
p
b
)
⋅
H
2
LWP= 
2
1
​
 Γ 
ad
​
 (T 
b
​
 ,p 
b
​
 )⋅H 
2
 
where 
H
=
z
ct
−
z
cb
H=z 
ct
​
 −z 
cb
​
  is the cloud thickness.

3.3 Equivalent Adiabaticity and Sub-Adiabatic Profiles
Real clouds are often sub-adiabatic due to entrainment, drizzle, and radiative effects. The equivalent adiabaticity 
α
eq
α 
eq
​
  (sometimes denoted 
β
β in the literature) is defined as the constant scaling factor that gives the same LWP as the observed profile:

α
eq
=
2
 
LWP
Γ
ad
(
T
b
,
p
b
)
⋅
H
2
α 
eq
​
 = 
Γ 
ad
​
 (T 
b
​
 ,p 
b
​
 )⋅H 
2
 
2LWP
​
 
with 
α
eq
=
1
−
β
α 
eq
​
 =1−β in some formulations. Observations suggest a degree of adiabaticity ranging from 0.6 to 0.2 for 200–600 m thick clouds.

3.4 Parameter Table for LWC/LWP Modeling
Parameter	Symbol	Typical Value / Range	Notes
Adiabatic condensation rate	
Γ
ad
Γ 
ad
​
 	~1–3 g m⁻³ km⁻¹	Depends on 
T
b
T 
b
​
 , 
p
b
p 
b
​
 
Cloud base height	
z
cb
z 
cb
​
 	0.5–2 km (marine BL)	From sounding or model
Cloud thickness	
H
H	100–1000 m	Shallow MBL clouds
Adiabaticity factor	
α
eq
α 
eq
​
 	0.2–1.0	1.0 = perfect adiabatic
LWP (adiabatic)	LWP	10–500 g m⁻²	From 
1
2
Γ
ad
H
2
2
1
​
 Γ 
ad
​
 H 
2
 
3.5 Applicability and Caveats
The adiabatic model is "plausibly more realistic for shallow marine boundary layer cloud regimes" than the homogeneous assumption. However, drizzle and cloud-top entrainment of dry air bias LWP retrievals away from adiabatic and toward homogeneous assumptions. The mean bias induced by drizzle-influenced profiles is on the order of 5–10 g/m², while cloud-top entrainment influence is smaller by about a factor of two.

4. Twilight and Civil Twilight Illuminance
4.1 Twilight Classification
Twilight is defined by the solar zenith angle 
ϕ
ϕ (angular distance from zenith). The three stages are:

Stage	Solar Zenith Angle Range	Description
Civil twilight	
90
∘
<
ϕ
≤
96
∘
90 
∘
 <ϕ≤96 
∘
 	Sun 0° to −6° below horizon
Nautical twilight	
96
∘
<
ϕ
≤
102
∘
96 
∘
 <ϕ≤102 
∘
 	Sun −6° to −12°
Astronomical twilight	
102
∘
<
ϕ
≤
108
∘
102 
∘
 <ϕ≤108 
∘
 	Sun −12° to −18°
The sky brightness decreases rapidly in the range 
90
∘
<
ϕ
<
108
∘
90 
∘
 <ϕ<108 
∘
 , with the transition from day to night occurring across these three stages.

4.2 Illuminance Decay Curve
For the ambient step at 0° solar elevation (open horizon item §4), a twilight illuminance curve is required. A practical parameterization uses an anchored exponential decay from the horizon:

E
twilight
(
alt
)
=
400
 
lx
×
exp
⁡
(
0.85
×
alt
)
E 
twilight
​
 (alt)=400lx×exp(0.85×alt)
for 
alt
≤
0
∘
alt≤0 
∘
 , where alt is solar altitude in degrees. This gives 400 lx at 0° (civil twilight boundary) down to approximately 2.5 lx at −6° (end of civil twilight).

4.3 Physical Basis: Hulburt's Twilight Model
Hulburt (1954) derived an analytical expression for twilight brightness based on Rayleigh scattering theory, primary scattering, and rocket-measured upper-air densities. The twilight brightness expression is:

I
(
λ
)
=
i
0
λ
σ
θ
λ
σ
λ
H
2
A
K
−
H
e
−
σ
λ
H
n
0
[
1
−
e
−
σ
λ
(
2
K
−
H
/
A
)
n
0
]
I(λ)=i 
0λ
​
  
σ 
λ
​
 
σ 
θλ
​
 
​
  
2AK−H
H
​
 e 
−σ 
λ
​
 Hn 
0
​
 
 [1−e 
−σ 
λ
​
 (2K−H/A)n 
0
​
 
 ]
where 
A
=
exp
⁡
(
r
θ
2
/
2
H
)
A=exp(rθ 
2
 /2H), 
θ
θ is the depression angle, 
H
H is the scale height (7 km), 
K
=
π
r
H
/
2
K= 
πrH/2
​
 , and 
n
0
n 
0
​
  is molecular density at the surface.

4.4 Ozone and Aerosol Influence
Twilight sky radiance and colour are calculated for single scattered radiation using spherically symmetric models of the Earth's atmosphere. Five different atmospheric models are used: pure molecular scattering, molecular + ozone absorption, and three models with varying aerosol concentrations. The twilight sky is sensitive to both ozone (which absorbs in the Chappuis band, affecting colour) and aerosols (which enhance scattering, particularly near the horizon).

4.5 Twilight Illuminance Reference Table
Solar Altitude	Twilight Stage	Approx. Illuminance	Source
0°	Civil twilight begins	400 lx	Anchored exponential model
−2°	Civil twilight	~80 lx	Exp decay
−4°	Civil twilight	~15 lx	Exp decay
−6°	Civil twilight ends	~2.5 lx	Exp decay
−12°	Nautical ends	~0.01 lx	Astronomical references
−18°	Astronomical ends	~0.001 lx	Night sky
5. Synthesis: Coupling Aerosol Optics to Cloud and Twilight Regimes
The four topics converge on a coherent physical picture:

Aerosol optics (Ångström 
α
α, 
β
β, Preetham 
T
T) govern clear-sky visibility and sky radiance.

Clouds (LWC profiles, LWP) are a distinct optical regime that should not be parameterized through aerosol turbidity.

Twilight is a transition regime where solar radiation undergoes long atmospheric paths, amplifying both aerosol and ozone effects.

Wet deposition couples the aerosol and cloud regimes: precipitation scavenges aerosols, reducing 
β
β and increasing visibility, while cloud LWC provides a quantitative link through the visibility–LWC relation.

The stylized parameters WET_TURBIDITY_GAIN and WATER_PATH_REFERENCE_M are grounded in the empirical visibility–LWC relation and the inverse PW–
β
β coupling, providing a physically sourced basis rather than a pure declaration.

References
Preetham, A. J., Shirley, P., & Smits, B. (1999). A practical analytic model for daylight. SIGGRAPH '99.

Zotti, G., Wilkie, A., & Purgathofer, W. (2007). A critical review of the Preetham skylight model. WSCG 2007.

Ångström, A. (1929). On the atmospheric transmission of sun radiation and on dust in the air. Geografiska Annaler.

Kasten, F., & Young, A. T. (1989). Revised optical air mass tables and approximation formula. Applied Optics, 28(22), 4735–4738.

Hulburt, E. O. (1954). Explanation of the brightness and color of the sky, particularly the twilight sky. Journal of the Optical Society of America.

Miller, D. J., et al. (2016). The impact of cloud vertical profile on liquid water path retrieval. JGR Atmospheres.

Bodhaine, B. A. (1999). Aerosol monitoring at Barrow, Alaska. NOAA Technical Report.

Haber, J., Magnor, M., & Seidel, H.-P. (2005). Physically-based simulation of twilight phenomena. ACM Transactions on Graphics, 24(4).