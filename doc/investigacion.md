---
title:
  Instituto Tecnológico de Costa Rica\endgraf\bigskip \endgraf\bigskip\bigskip\
  Investigación - Generación de Código \endgraf\bigskip\bigskip\bigskip\bigskip
author:
  - José Morales Vargas, carné 2019024270
  - Alejandro Soto Chacón, carné 2019008164
  - José Alejandro Chavarría Madriz, carné 2019067306
  - Natalia González Bermúdez, carné 2019165109
date: \bigskip\bigskip\bigskip\bigskip Area Académica de\endgraf Ingeniería en Computadores \endgraf\bigskip\bigskip\ Lenguajes, Compiladores \endgraf e intérpretes (CE3104) \endgraf\bigskip\bigskip Profesor Marco Hernández Vásquez \endgraf\vfill  Semestre I
header-includes:
  - \setlength\parindent{24pt}
  - \usepackage{url}
lang: es-ES
papersize: letter
classoption: fleqn
geometry: margin=1in
fontsize: 12pt
fontfamily: sans
linestretch: 1.15
bibliography: doc/investigacion.bib
csl: /home/josfemova/UsefulRepos/styles/ieee.csl
nocite: |
  @crcc, @icld
...

\maketitle
\thispagestyle{empty}
\clearpage
\tableofcontents
\pagenumbering{roman}
\clearpage
\pagenumbering{arabic}
\setcounter{page}{1}

# Objetivos

Explicar los conceptos básicos detrás de la etapa de generación de código en un compilador actual(?)
Describir los pasos mínimos requeridos a la hora de generar código en un compilador
Explicar una implementación ad hoc para generación de código en tiempo de ejecución
Brindar un ejemplo conceptual para ilustrar el proceso de generación de código

# Introducción

En la presente investigación se pretende ahondar en la etapa llevada a cabo durante el proceso de compilación, conocida como generación de código. Para ello se comenzará con una explicación de los conceptos básicos a manejar para comprender a cabalidad esta etapa, a su vez se explicarán los pasos mínimos más comunes requeridos al generar código, esto con el fin de brindar una comprensión global del proceso de manera simplificada y teórica. Esta etapa de la investigación se basa principalmente en “Introduction to Compilers and Language Design”[@icld].

Adicionalmente, valiéndonos del conocimiento en el primer segmento, se expondrá un procedimiento alternativo para la etapa de generación de código. Dicho procedimiento consiste en la generación de código en tiempo de ejecución, un método que pretende generar código especializado creado rápidamente de manera dinámica, mediante el uso de templates y código precompilado. Para esta sección se ha utilizado como base “Compiling for Runtime Code Generation”[@crcc].

Finalmente se procederá a describir un ejemplo conceptual completo para reforzar los conceptos y crear una mayor cercanía con los temas, principalmente, de la primera sección de la investigación(?).

# Generación de código y diseño de lenguaje

# Generación de código en tiempo de ejecución

El artículo cubre la técnica de _Generación de código en tiempo de ejecución_ o RTCG por sus siglas en inglés. La última es una técnica de procesamiento de código que permite la generación dinámica de código compilado durante el tiempo de ejecución mismo.

La técnica de compilado en tiempo de ejecución debe hacer un balance entre dos factores fundamentales: tiempo de compilación y optimización de código.

El artículo cubre este tema usando como ejemplo un lenguaje desarrollado por los autores llamado Cyclone, el cual es estáticamente tipado. Este lenguaje se transpila a una variante tipada de lenguaje ensamblador. Se menciona que durante el proceso de transpilado, anteriormente, para el código que era generado en tiempo de ejecución no tendría optimización alguna, lo que aumentaría el costo en recursos computacionales, y podría contrarrestar la ventaja de la generación de código de forma dinámica.

Se mencionan varios acercamientos para poder conseguir generación de código en tiempo de ejecución, entre ellas un modelo basado en templates sin optimizaciones, otro con manipulación de strings que son posteriormente compilados en tiempo de ejecución por un compilador convencional por lo tanto asegurando una optimización de código adecuada. Los autores deciden utilizar un tercer acercamiento, el cual consiste en analizar el flujo de datos de las funciones de generación de código de forma que se pueda obtener un grafo que caracteriza de forma general los posibles productos de la función generadora, esto permite realizar optimizaciones sobre los elementos de la función producto.

Para guiar en el tema, los autores introducen su lenguaje Mini-Cyclone, la variante experimental del lenguaje cyclone que se mencionó anteriormente. Este lenguaje no se compila directamente a lenguaje máquina, sino que su compilado es a un lenguaje intermedio que los autores llaman CIR (Cyclone Intermediate Representation). Este lenguaje mencionan es relativamnete estándar, un IR de bajo nivel y basado en bloques.

Para comprender mejor el árticulo es escencial notar en especial algunos componentes del lenguaje que son fundamentales:

- `codegen` Es la principal expresión generadora de funciones de manera dinámica, es decir, la expresión se utiliza para crear las funcinoes generadoras de código dentro del programa. En mini-Cyclone, esta expresión genera la función y retorna un puntero a la misma.
- `fill` Es una expresión que permite evaluar expresiones dentro de una sección de `codegen` a partir de parámetros pasados anteriormente a codegen, es decir, aquello que englobe fill será evaluado antes de la generación de código de la función producto.
- `cut` Es una sentencia que se utiliza para remover las partes de código utilizadas para generación que no son necesarias en la instancia generada.
- `splice` Sentencia que permite conservar una sección de código de la función generadora dentro de la función generada, dado que la sección de código a conservar se encuentre envuelta dentro de un cut.



## Ejemplos (?)

# Conclusiones generales

# Conclusiones específicas

# Bibliografía

::: {#refs}
:::
