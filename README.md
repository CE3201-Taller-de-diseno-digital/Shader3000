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
monofont: "Noto Sans Mono"
linestretch: 1.15
bibliography: bibliografia.bib
csl: /home/josfemova/UsefulRepos/styles/ieee.csl
nocite: |
  @lexyacc, @rustbook, @esp8266-techref, @esp8266-pinout, @xtensa-assembly, @shiftregister-datasheet, @multiplexor-datasheet
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

Previo a que se diera la especificación, se experimentó con la posibilidad de una tercera propuesta relativamente distinta, en búsqueda de enriquecer la experiencia pedagógica. La propuesta consistía en desarrollar todos los componentes del proyecto utilizando únicamente el lenguaje de programación Rust. Dicho lenguaje es relativamente nuevo, pero tiene ciertas características que lo hacen particularmente interesante. El lenguaje permite un nivel de control similar a C++, pero ha tomado nota de los problemas que presentan otros lenguajes de bajo nivel, por lo que ha buscado inspiración en varios lugares. Presenta características de lenguajes relativamente más modernos como python, y provee algunas funcionalidades inspiradas en lenguajes funcionales. Uno de los aspectos más esenciales a tomar en cuenta es que Rust es un lenguaje cuya prioridad es la correctitud y la "seguridad" de código, lo que suele minimizar el tiempo utilizado en resolución de problemas. El equipo encontró interesante trabajar en dicho lenguaje y se consideró que podría ser más productivo para nuestro proceso de aprendizaje el experimentar con Rust en vez de tomar un camino más familiar. La expresividad del lenguaje también fue un factor a tomar en cuenta, y se consideró que Rust presentaba un balance adecuado entre capacidades y facilidad para desarrollo. 

Anteriormente se mencionaba que se "experimentó" con la propuesta. Esto es porque inicialmente no se sabía que tan factible sería el uso de Rust para la producción de código a a ser ejecutado en el microcontrolador. Arduino es basado exclusivamente en C++, y todas las funcionalidades dependen de código no portable a Rust. Afortunadamente se encontró una iniciativa experimental para dar soporte de Rust a los microcontroladores de Espressif[@mabez], principalmente los MCU ESP32 y ESP8266. Para poder ejecutar código Rust en un microcontrolador ESP8266 fue necesario entonces compilar una versión especial de LLVM que provee soporte a la arquitectura de Xtensa (usada en los MCU mencionados) [@rust-xtensa], lo que al mismo tiempo requería descargar el toolchain de Espressif para desarrollo en el ESP8266 [@esp8266-toolchain]. Lo anterior ya permite trabajar sobre el microcontrolador, sin embargo, todavía era necesario una biblioteca de abstracción de hardware (equivalente a las que se utilizan en arduino). Para esto se recurrió a la biblioteca (o "crate" en términos de Rust) esp8266-hal [@esp8266-hal]. Adicionalmente, se recurrió a una herramienta de cargado de código al ESP8266 llamada esptool[@esptool]. La misma también asiste el proceso de ejecución del ambiente de desarrollo implementado. Utilizando el ambiente descrito anteriormente, se logró comprobar la viabilidad de desarrollar el proyecto completamente en Rust. Si bien la biblioteca de abstracción de hardware es algo reducida, se consideró que era suficiente para los requisitos del proyecto. 

La complejidad del proceso para poder ejecutar código Rust sobre el ESP8266 sí presentó una duda particular. El equipo ya sabía que era factible usar Rust, ¿pero sería más provechoso que utilizar C++ con arduino? 

Finalmente se consideró que la dificultad de uso de la plataforma para ejecución de código era un precio aceptable a pagar por la practicidad que ofrece trabajar en un lenguaje como Rust. Otro factor que se tomó en cuenta era que varias herramientas de Rust facilitarían la integración del código completo del proyecto, lo que daría una reducción neta del tiempo de desarrollo.

## Microcontrolador a utilizar

## Máquina virtual vs generación de código máquina

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
