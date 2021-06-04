### Previo al proyecto

Experimentación con instalación de ambiente de programación para el uso de Rust como el lenguaje para programación del MCU

### 12 de mayo

- El profesor entrega la especificación del proyecto

### 13 de mayo

- Se realiza una primera reunión para realizar la coordinación inicial del equipo

### 14 de mayo

### 15 de mayo

### 16 de mayo

### 17 de mayo

### 18 de mayo

Entrega de componentes a compañeros de proyecto
Prueba de algunos scripts básicos para ejecutar en el mcu

### 19 de mayo

Reunión
Se resuelven dudas mayores referentes al proyecto

### 20 de mayo

### 21 de mayo

### 22 de mayo

### 23 de mayo

Compañeros de trabajo preparan ambiente de programación
Se agregan fuentes consultadas hasta el momento a la bibliografía
Se agrega documentación sobre alternativas de solución y justificación 

### 24 de mayo

Se completa documentación sobre alternativas de solución y justificación de la implementada

### 25 de mayo


### 26 de mayo

Se realizan algunas secciones del parser, aunque el trabajo llega a ser más conceptual que aplicable. Observaciones encontradas en el camino se le hacen saber a otros compañeros de trabajo.

### 27 de mayo

Se implementan algunas funciones básicas de parseo, aunque la velocidad de desarrollo de este módulo es menor a la que se desearía. 
Se encuentran dificultades particulares con parseo de algunas expresiones recursivas.

### 28 de mayo

Tomando en cuenta las habilidades de los mimebros, se decide realizar un cambio de tareas de forma que mi persona pasará a trabajar en el desarrollo de la programación del sistema embebido a utilizar. Las ventajas en tiempo que esto implica son significativas, por lo que el equipo decide proceder de esa manera.

### 29 de mayo

Se realiza una reunión con la compañera Natalia para poder depurar algunos problemas aparentes con el montaje de la matriz de leds, sin embargo, se logra descartar la existencia de un problema mayor. El problemas se debía a una colocación de pines inadecuada. Una vez corregido esto, la matriz de leds funcionó perfectamente

### 30 de mayo

Se implementan diferentes módulos del embebido para dibujo de la matriz de leds. Por ahora se busca completar estas funciones básicas.
Se nota la urgencia de encontrar una forma de activar las interrupciones por timers de hardware, puesto que la lógica de como funciona el proceso compilado depende de que se pueda ejecutar código de manera paralela al código compilado.
Se realiza una investigación a fondo de los diferentes módulos siendo utilizados para abstraer el harware, en particular los crates esp8266_hal, esp8266 y xtensa-lx. Se analizó tanto como documentación disponible, así como el código de dichos crates.

### 31 de mayo

Se experimenta un poco con la implementación de los timers de hardware. La documentación respecto a esto es muy reducida, por lo cual fue algo complicado.
Se logró encontrar una forma de activar las interrupciones por hardware

### 1 de junio

Se encuentran algunos problemas de inconsistencia con el manejo de los timers. Por alguna razón no definida, hay síntomas de presencia de deadlocks, sin embargo no se puede afirmar con seguridad. Existen posibilidades de malfuncionamientos como producto de otros elementos. 
Se decide explorar la posibilidad de que la causa de los problemas de inconsistencia sea un dealock, por lo que se procede a intentar el uso de SpinLocks disponibles en uno de los crates utilizados. El código no servía adecuadamente sin ajustes, por lo cuál se consultó con el autor del módulo (Mabez) sobre el procedimiento a llevar a cabo para poder utilizar esta variedad de mutex.

### 2 de junio

Se llega a la conclusión de que los problemas encontrados no son causados por deadlocks, puesto que se experimenta de forma aislada en un ambiente distinto del proyecto y las rutinas sí parecen funcionar cuando se construyen desde ese ambiente aislado. 
El funcionamiento de los timers se detecta como algo inconsistente, por lo cual se procede a investigar como configurar el timer de forma que se ajuste a las necesidades del proyecto. Esto concluye en la implementación de una condiguración de bajo de nivel del timer.
Se utilizó todo el día en depurar un problema con la forma en la que se escribe el programa en el esp8266

### 3 de junio

Se solucionan problemas que impedían un correcto funcionamiento del esp8266
Se limpian algunas secciones de código innecesarias y se cambian algunas configuraciones para poder evitar los problemas detectados.
El problema encontrado era un problema de configuración de memoria del MCU, el cual se daba por una construcción incorrecta de una de las dependencias del proyecto, esto pues se utilizaba una versión mucho más antigua a la funcional actual.
Se le entrega un esp8266 al compañero Alejandro para que pueda experimentar con el microcontrolador él también cuando se encuentren conflictos extraños, esto pues es algo inefectivo tratar de resolver el problema entre dos personas cuando solo una tiene acceso al equipo físico. 

### 4 de junio

Se agregan funcionalidades de blink para milisegundos, segundos y minutos en el sistema embebido.

### 5 de junio

### 6 de junio

### 7 de junio

### 8 de junio

### 9 de junio

### 10 de junio

### 11 de junio


