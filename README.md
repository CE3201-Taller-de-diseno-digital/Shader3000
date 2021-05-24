---
title:
  Instituto Tecnológico de Costa Rica\endgraf\bigskip \endgraf\bigskip\bigskip\
  Proyecto Final - AnimationLed \endgraf\bigskip\bigskip\bigskip\bigskip
author:
  - José Morales Vargas, carné 2019024270
  - Alejandro Soto Chacón, carné 2019008164
  - José Alejandro Chavarría, carné
  - Natalia, carné
date: \bigskip\bigskip\bigskip\bigskip Area Académica de\endgraf Ingeniería en Computadores \endgraf\bigskip\bigskip\ Lenguajes, Compiladores \endgraf e intérpretes (CE3104) \endgraf\bigskip\bigskip Profesor Marco Hernández Vásquez \endgraf\vfill  Semestre I
header-includes:
  - \setlength\parindent{24pt}
  - \usepackage{url}
lang: es-ES
papersize: letter
classoption: fleqn
geometry: margin=1in
#fontfamily: sans
fontsize: 12pt
fontfamily: sans
linestretch: 1.15
bibliography: bibliografia.bib
csl: /home/josfemova/UsefulRepos/styles/ieee.csl
nocite: |
  @lexyacc
...

\maketitle
\thispagestyle{empty}
\clearpage
\tableofcontents
\pagenumbering{roman}
\clearpage
\pagenumbering{arabic}
\setcounter{page}{1}

# CE3104-AnimationLed

# Diagrama de arquitectura

# Alternativas de solución consideradas y justificación de la seleccionada

## Lenguaje de programación

La primera decisión sobre la arquitectura del proyecto fue sobre el o los lenguajes de programación a utilizar. El grupo se había formado previamente a la entrega de la especificación, por lo cual esto fue discutido de forma temprana. Habían al menos tres propuestas principales. La primera era realizar un proyecto completamente utilizando solo C/C++, esto ya que las herramientas a las que se gravitan por defecto trabajan de una u otra manera con estos lenguajes (Arduino, Yacc, Lex). La segunda propuesta era una derivada de la anterior: compilador y programación del MCU sería realizada utilizando C/C++, pero el editor de código se haría en un lenguaje de un nivel un poco más alto para facilitar la creación de la interfaz gráfica.

Previo a que se diera la especificación, se experimentó con la posibilidad de una tercera propuesta relativamente distinta, en búsqueda de enriquecer la experiencia pedagógica. La propuesta consistía en desarrollar todos los componentes del proyecto utilizando únicamente el lenguaje de programación Rust. Dicho lenguaje es relativamente nuevo, pero tiene ciertas características que lo hacen particularmente interesante. El lenguaje permite un nivel de control similar a C++, pero ha tomado nota de los problemas que presentan otros lenguajes de bajo nivel, por lo que ha buscado inspiración en varios lugares. Presenta características de lenguajes relativamente más modernos como python, y provee algunas funcionalidades inspiradas en lenguajes funcionales. Uno de los aspectos más esenciales a tomar en cuenta es que Rust es un lenguaje cuya prioridad es la correctitud y la "seguridad" de código, lo que suele minimizar el tiempo utilizado en resolución de problemas. El equipo encontró interesante trabajar en dicho lenguaje y se consideró que podría ser más productivo para nuestro proceso de aprendizaje el experimentar con Rust en vez de tomar un camino más familiar. La expresividad del lenguaje también fue un factor a tomar en cuenta, y se consideró que Rust presentaba un balance adecuado entre capacidades y facilidad para desarrollo. El recurso principal de consulta fue el mismo manual del lenguaje [@rustbook].

Anteriormente se mencionaba que se "experimentó" con la propuesta. Esto es porque inicialmente no se sabía que tan factible sería el uso de Rust para la producción de código a a ser ejecutado en el microcontrolador. Arduino es basado exclusivamente en C++, y todas las funcionalidades dependen de código no portable a Rust. Afortunadamente se encontró una iniciativa experimental para dar soporte de Rust a los microcontroladores de Espressif [@mabez], principalmente los MCU ESP32 y ESP8266. Para poder ejecutar código Rust en un microcontrolador ESP8266 fue necesario entonces compilar una versión especial de LLVM que provee soporte a la arquitectura de Xtensa [@rust-xtensa](usada en los MCU mencionados), lo que al mismo tiempo requería descargar el toolchain de Espressif para desarrollo en el ESP8266 [@esp8266-toolchain]. Lo anterior ya permite trabajar sobre el microcontrolador, sin embargo, todavía era necesario una biblioteca de abstracción de hardware (equivalente a las que se utilizan en arduino). Para esto se recurrió a la biblioteca (o "crate" en términos de Rust) esp8266-hal [@esp8266-hal]. Adicionalmente, se recurrió a una herramienta de cargado de código al ESP8266 llamada esptool [@esptool]. La misma también asiste el proceso de ejecución del ambiente de desarrollo implementado. Utilizando el ambiente descrito anteriormente, se logró comprobar la viabilidad de desarrollar el proyecto completamente en Rust. Si bien la biblioteca de abstracción de hardware es algo reducida, se consideró que era suficiente para los requisitos del proyecto. La biblioteca también da herramientas suficientes para implementar algunas funcionalidades especificadas en el documento de referencia técnica del ESP8266 [@esp8266-techref].

La complejidad del proceso para poder ejecutar código Rust sobre el ESP8266 sí presentó una duda particular. El equipo ya sabía que era factible usar Rust, ¿pero sería más provechoso que utilizar C++ con arduino? Se consideró que la dificultad de uso de la plataforma para ejecución de código era un precio aceptable a pagar por las ventajas que ofrece trabajar en un lenguaje como Rust. Otro factor que se tomó en cuenta era que varias herramientas de Rust facilitarían la integración del código completo del proyecto, lo que daría una reducción neta del tiempo de desarrollo y pruebas.

## Componentes electrónicos

El equipo tenía varios microcontroladores a disposición. Se consideraron principalmente tres opciones, ESP32, ESP8266 y arduino pro micro. Se hizo un balance entre facilidades para desarrollar, características ofrecidas y disponibilidad. La mejor combinación de las características mencionadas se encontró en el ESP8266. La disponibilidad era especialmente importante, y del ESP8266 el equipo posee cuatro muestras distintas en total. Simultáneamente cada sub-equipo solo tiene un MCU operacional, pero mantiene otro como un respaldo en caso de errores que puedan dañar el dispositivo. Datos sobre funcionalidades de los puertos disponibles en el dispositivo fueron obtenidas de [@esp8266-pinout]

Leds y resistencias fueron seleccionas según disponibilidad, aunque para el valor específico de las últimas se prefirió usar algo que evitara la posibilidad de que se quemara cualquiera de los leds, en detrimento de la luminosidad posible en la configuración.

Para control de la matriz, se recurrió al uso de un registro de corrimiento 74lS164N, cuya hoja de datos puede ser encontrada en [@shiftregister-datasheet], para registrar los leds activos de cada columna según fila. Esta decisión fue más por disponibilidad que otro factor, aunque una aspecto conveniente del dispositivo es que es fácilmente reemplazable por registros de corrimiento fáciles de conseguir (como el 72HC595). Para controlar el encendido secuencial de cada fila se utilizó un multiplexor 74HC4067, esto porque es más simple de manipular que un registro de corrimiento y la disponibilidad de registros de corrimiento era insuficiente como para permitir los dos montajes requeridos (experimental y entrega). La hoja de datos del multiplexor puede ser encontrada en [@multiplexor-datasheet].

## Máquina virtual vs generación de código máquina

Para la ejecución de código en el microcontrolador habían dos opciones principales: programar una máquina virtual que ejecute los comandos, o producir código de máquina que se ejecute directamente sobre la plataforma. Para decidir cual acercamiento tomar se compararon aspectos de complejidad conceptual, fuentes disponibles para guiarse en la implementación y complejidad de ejecución.

El acercamiento de implementar una máquina virtual con operaciones propias se consideró menos complejo a nivel conceptual, aunque su complejidad de implementación era mayor que la de una generación de código directa. Entre las ventajas principales de este método está el poder definir una interfaz de alto nivel, que permite ejecución relativamente directa de algunas operaciones. Lo anterior parecería indicar que la complejidad de implementación es reducida, sin embargo, si se quisiese poder correr el código por completo en el microcontrolador sin necesidad de estar conectado a un programa intermediario, ciertas dificultades mayores empiezan a presentarse, e incluso con un intérprete intermedio, surgen dificultades similares si no es que iguales. En primer lugar, se debía implementar por completo un set de instrucciones que no solo contiene instrucciones de alto nivel, sino que también debe ser capaz de interpretar comandos como saltos condicionales, comparaciones, operaciones, entre otros. Lo anterior significaba que el equipo debía asumir una tarea adicional de definir una infraestructura virtual para poder interactuar con el microcontrolador. Ahora, no solo debía definirse dicha máquina virtual, sino que debía implementarse en el mismo código del programa. 

En contraste, el acercamiento de producir directamente código ejecutable tiene una complejidad aparente mayor, sin embargo provee ventajas innegables, entre ellas que prescinde de la definición de una máquina virtual. Este acercamiento es algo diferente en cuanto a funcionamiento. En tiempo de compilación, en vez de crear un ejecutable para la máquina virtual, se crea un ejecutable para el procesador del microcontrolador mismo, y las funcionalidades complejas se pueden acceder por medio de una biblioteca previamente definida en Rust. El reto principal de este acercamiento es la producción del código ensamblador para arquitectura Xtensa, con la cual el equipo no tiene experiencia previa, aunque un miembro sí tiene experiencia previa trabajando con lenguajes de ensamblador de otras arquitecturas. Cabe notar que la dificultad del proceso consiste en la generación de los procedimientos a partir de las instrucciones primitivas, pero la documentación de estas primitivas es sumamente directa y simple. Dicha documentación puede ser encontrada en [@xtensa-assembly].

Después de una reunión para decidir sobre este aspecto de la implementación del proyecto, el equipo decidió seguir el acercamiento de producción de código de máquina directamente. Se consideró que si bien la complejidad aparente de este método era mayor, su complejidad real y tiempo de implementación sería menor. Además se consideró que sería más provechoso para el proceso de aprendizaje la generación de código de máquina real. No puede omitirse el hecho de que también entra en juego un aspecto de preferencia personal. Al equipo le pareció más entretenido y atractivo el implementar un compilador a código de máquina real, que implementar un transpilador a un código intermedio inventado que no se utiliza en ambientes de trabajo reales. 

## Uso de Lex y Yacc



# Problemas conocidos

# Actividades realizadas por estudiante

# Problemas encontrados

1. Problema 1

   - _Descripción_: lorem ipsum

   - _Intentos de solución_:

     1. a
     2. b
     3. c

   - _Solución encontrada_: lorem ipsum

   - _Conclusiones_:
     1. a
     2. b
     3. c
   - _Recomendaciones_:
     - lorem ipsum
   - _Bibliografía_:
     - lorem ipsum

# Conclusiones y Recomendaciones del Proyecto

## Conclusiones

- a
- b
- c

## Recomendaciones

- a
- b
- c

# Bibliografía

::: {#refs}
:::
