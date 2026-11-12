# Compatibility / Compatibilidad

**English** — Only the **Epson XP-530** has been verified on real hardware. The models
below are **untested**, but the embedded database has a complete profile for them
(service block, waste counter addresses and reset table), so InkReset should be able to
read and reset their counters. A further ~660 models only have a service block: InkReset
can connect, but cannot read or reset their counters yet.

**Help us test.** If you own one of these printers, run a read and the safe write test,
then [open an issue](https://github.com/datacas/inkreset/issues/new) with the model, the
`MDL:` string the app shows and what worked. Verified models will be moved to the
tested list.

**Español** — Solo la **Epson XP-530** está verificada en hardware real. Los modelos de
abajo **no están probados**, pero la base de datos integrada tiene un perfil completo para
ellos (bloque de servicio, direcciones de los contadores y tabla de reinicio), así que
InkReset debería poder leer y reiniciar sus contadores. Otros ~660 modelos solo tienen
bloque de servicio: InkReset puede conectar, pero aún no leer ni reiniciar sus contadores.

**Ayúdanos a probar.** Si tienes una de estas impresoras, haz una lectura y la prueba de
escritura segura y [abre una incidencia](https://github.com/datacas/inkreset/issues/new)
con el modelo, la cadena `MDL:` que muestra la app y qué ha funcionado.

## Verified / Verificadas

- **XP**: XP-530

## Expected to work, untested / Deberían funcionar, sin probar (312)

- **Artisan**: Artisan 1430, Artisan 730, Artisan 837
- **EP**: EP-10VA, EP-306, EP-4004, EP-706A, EP-707A, EP-708A, EP-709A, EP-775A, EP-776A, EP-777A, EP-804A, EP-805A, EP-806A, EP-807A, EP-808A, EP-904A, EP-904F, EP-905A, EP-905F, EP-906A, EP-906F, EP-907A, EP-907F, EP-976A3, EP-977A3, EP-978A3, EP-979A3, EP-M570T
- **ET**: ET-1110/1111/1113/1115, ET-1112/1114/1116/1118, ET-14000, ET-1810/1811/1813/1815, ET-1812/1814/1816/1818, ET-2500, ET-2550, ET-2600, ET-2610, ET-2650, ET-2700/2701/2703/2705, ET-2702/2704/2706/2708, ET-2710/2711/2713/2715, ET-2712/2714/2716/2718, ET-2720/2721/2723/2725, ET-2722/2724/2726/2728, ET-2750/2751/2756, ET-2760, ET-2800/2801/2803/2805, ET-2802/2804/2806/2808, ET-2810/2811/2813/2815, ET-2812/2814/2816/2818, ET-2820/2821/2823/2825, ET-2822/2824/2826/2828, ET-2850/2851/2856, ET-3600, ET-4500, ET-4550, ET-4700, ET-4800, ET-M1100, ET-M1120, ET-M2120
- **EW**: EW-052A, EW-M571T, EW-M660FT
- **L**: L110/L111, L1110, L1118, L1119, L120, L1210/L1216/L1218/L1219, L1250/L1256/L1258/L1259, L130, L1300, L132, L1800, L210/L211, L220, L222, L300/L301, L3050, L3060, L3070, L310, L3100/L3101, L3104/L3105, L3106/L3107, L3108/L3109, L3110/L3111, L3114/L3115, L3116/L3117, L3118/L3119, L312, L3150/L3151, L3152/L3153, L3156/L3158, L3160/L3166/L3168, L3161/L3163/L3165, L3200/L3211/L3213/L3215, L3206/L3207/L3208/L3209, L3210/L3211/L3213/L3215, L3216/L3217/L3218/L3219, L3250/L3251/L3253/L3255, L3256/L3527/L3258/L3259, L3260/L3261/L3263/L3265, L3266/L3267/L3268/L3269, L350/L351, L355, L360, L362, L364, L365, L366, L375, L380, L382, L385, L386, L395, L396, L405, L4150/L4156/L4158, L4160/L4167/L4168, L4250/L4251/L4253/L4255, L4256/L4257/L4258/L4259, L4260/L4261/L4263/L4265, L4266/L4267/L4268/L4269, L455, L456, L475, L485, L486, L495, L5190/L5196/L5198, L5290/L5296/L5298, L550/L551, L555/L556, L565, L566, L575, L605/L606, L655/L656, L805, L810, L850
- **M**: M100, M105, M1100, M1120, M200, M205, M2110/M2118/M2119, M2120/M2128/M2129
- **ME**: ME Office 530, ME Office 570, ME-10, ME-100/ME-101, ME-300/ME-301, ME-302/ME-303, ME-400/ME-401, ME-85
- **PM**: PM-A900, PM-A950, PM-A970, PM-G800, PM-G820, PM-T990
- **PX**: PX-045A, PX-046A, PX-047A, PX-048A, PX-049A, PX-105, PX-1200, PX-1600F, PX-1700F, PX-404A, PX-405A, PX-434A, PX-435A, PX-436A, PX-437A, PX-505F, PX-535F, PX-5500, PX-G5000, PX-G5100, PX-K150, PX-M650A, PX-M650F, PX-M680F, PX-S170T, PX-S170UT
- **Stylus**: Stylus BX935FWD, Stylus NX230, Stylus NX330, Stylus NX430, Stylus PX730WD, Stylus PX830FWD, Stylus Photo 1430, Stylus Photo 1500W, Stylus R1800, Stylus R800, Stylus RX700, Stylus SX230, Stylus SX235W, Stylus SX430W, Stylus SX435W, Stylus SX440W, Stylus TX230, Stylus TX235, Stylus TX235W, Stylus TX430W, Stylus TX435W, Stylus TX730WD, Stylus TX830FWD
- **WF**: WF-2010/2011/2018, WF-2510/2511/2518, WF-2520/2521/2528, WF-2530/2531/2538, WF-2540/2541/2548, WF-2630/2631/2635, WF-2650/2651/2655, WF-2660/2661/2665, WF-2750/2751/2755, WF-2760/2761/2765, WF-3720/3723/3725, WF-3730/3733/3735, WF-7010, WF-7011, WF-7012, WF-7015, WF-7018, WF-7510, WF-7511, WF-7515, WF-7520, WF-7521, WF-7525, WF-M1030, WF-M1560
- **WorkForce**: WorkForce 845
- **Workforce**: Workforce ST-2000, Workforce ST-C2100
- **XP**: XP-100, XP-102/103, XP-200, XP-201/204/208, XP-202/203/206, XP-205/207, XP-210, XP-2100/2101/2105, XP-211/214/216, XP-212/213, XP-215/217, XP-2150/2151/2155, XP-220/221, XP-225, XP-230/231, XP-235, XP-240/241/242, XP-243/245/247, XP-255/257, XP-30/33, XP-300/301, XP-302/303/305/306, XP-310/311, XP-312/313/315/316, XP-320/324, XP-322/323/325, XP-330/334, XP-332/333/335, XP-340/344, XP-342/343/345, XP-352/355, XP-400/401, XP-402/403/405/406, XP-410/411, XP-412/413/415/416, XP-420/421/424, XP-422/423/425, XP-430/431/434, XP-432/433/435, XP-440/441/446, XP-442/443/445, XP-452/455, XP-510, XP-520, XP-540, XP-55, XP-600/601/605, XP-610/611/615, XP-620/621/625, XP-630/631/635, XP-640/641/645, XP-700/701/702, XP-710, XP-7100, XP-720/721, XP-750, XP-760, XP-800/801/802, XP-810, XP-820/821, XP-830, XP-850, XP-860, XP-900, XP-950, XP-960
