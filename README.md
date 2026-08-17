# DCtuneRojo
Monitoreo de ECU para VW Motor 4cilindros carburador y fuelinyectio
DCTuneR / LibreTune / DC laboratory 
El archivo proporcionado contiene el código fuente de DCTuneR (que incorpora libretune-core y libretune-app), una suite moderna y multiplataforma para la sintonización (tuning), registro de datos (datalogging) y análisis de unidades de control de motor (ECU) diseñada para sistemas de gestión de motores aftermarket (como Speeduino). Funciona como una alternativa de código abierto al software de sintonización tradicional como TunerStudio.
Arquitectura Central (libretune-core)
Construida en Rust, la librería central gestiona la comunicación de bajo nivel con la ECU, el procesamiento de datos y funciones avanzadas:
Protocolo y comunicación con la ECU: Gestión de puertos seriales, construcción de paquetes, descubrimiento, despacho de comandos, mapeo de memoria y registros de sombra (shadow registers).
Análisis de INI y configuración: Análisis robusto de archivos de definición de ECU (.ini), constantes, canales de salida, expresiones, indicadores (gauges), tablas y conversiones de unidades.
Gestión de proyectos y calibraciones (Tunes): Manejo de archivos de calibración, comparación de diferencias (diffing), migraciones, canales matemáticos, control de versiones y sincronización con repositorios en línea.
Motor de tiempo real y Autotune: Evaluación de expresiones en tiempo real, algoritmos de autotune, detección de anomalías, predictores, enriquecimiento por aceleración y capacidades de análisis VE (VE analyze).
Extensibilidad e integración de IA: Soporte integrado para secuencias de comandos en Lua, un sistema de complementos (plugins), scripts de acciones y un marco de agentes de IA con compatibilidad para múltiples proveedores de LLM (OpenAI, Anthropic, Google).
Aplicación Frontend (libretune-app)
Construida con TypeScript y tecnologías web/de escritorio modernas:
Editores de tablas interactivos: Editores de tablas 2D y 3D completos (TableEditor2D, TableEditor3D) con cuadrículas visuales, menús contextuales, barras de herramientas y tablas de vista previa de lambda.
Paneles y lecturas en vivo: Lecturas de canales en tiempo real, almacenamiento de registros gráficos (graph log stores), diseños de paneles personalizados y editores de curvas.
Interfaz de sintonización y diálogos: Gestión integral de diálogos para crear nuevas calibraciones, grabar (burn) configuraciones, cargar y guardar archivos, configuración de conexiones y gestión de sesiones en vivo de AutoTune.
CI/CD y Automatización
Incluye configuraciones automatizadas de flujos de trabajo de GitHub Actions (ci.yml, release.yml, nightly.yml) para la compilación, pruebas y lanzamientos en diferentes plataformas de destino.
AUTOR:DC-Laboratory.
