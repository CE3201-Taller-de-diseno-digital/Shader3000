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

# Generación de código

## Generación de código y diseño de lenguaje

## Generación de código en tiempo de ejecución

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



## Ejemplo
Para ilustrar el proceso de generación de código se analizará un ejemplo sencillo presentando el significado del código traducido así como su origen y el proceso que lo genera.
Analicemos el siguiente código:
Código fuente: 
```
i = 0
while i < 10:
    putc(65 + i)
    i = i + 1
```
Código generado:
``` 
loop:
        pushq   $0              # allocate stack space for "i"
        pushq   %rbp            # save and setup frame pointer
        movq    %rsp, %rbp
        movq    $0, 8(%rbp)     # i = 0
loop_1_while:
        movq    $10, %rdx       # rax = 1 if i < 10 else 0
        movq    8(%rbp), %rax
        cmpq    %rdx, %rax
        movq    $0, %rax
        jnl     loop_3_less
        incq    %rax
loop_3_less:
        cmpq    $0, %rax           # if bool is zero, break
        jz      loop_2_break        
        movq    8(%rbp), %rdx      # 65 + i
        movq    $65, %rax          
        addq    %rdx, %rax         
        movq    %rax, %rdi         
        movq    stdout(%rip), %rsi # putc()
        call    putc               
        movq    $1, %rdx           # i = i + 1
        movq    8(%rbp), %rax      
        addq    %rdx, %rax          
        movq    %rax, 8(%rbp)      
        jmp     loop_1_while        
loop_2_break:
        popq    %rbp               # restore frame pointer
        leaq    8(%rsp),%rsp       # deallocate stack space for "i"
        ret                        # return to caller
```
Analicemos el primer fragmeto:
[fig 1]
`i = 0` es una declaración, por lo que para generar el código en x86 se utiliza el procedimiento anteriormente especificado para la generacion de declaraciones. Es especificamente las dos primeras lineas `pushq` crean un espacio para i y un puntero. Las siguiente dos, `movq`, asignan los valores a ambas variables.

[fig 2]
`while i < 10:` es un “statement” que genera un loop, para generar el código se utilizan los procedimientos descritos en “Generación de estructuras”. Para este caso únicamente la declaración del while y su condición generan el código de la fig 2. Las primeras 3 líneas se utilizan para evaluar la condición.
`cmpq` y no activa un flag si i < 10, de lo contrario si lo activa. `jnl` es un salto condicional (Salto si no es menos). Si el flag está activo salta directamente a `loop_3_less`, tal como se aprecia en la figura 5. Si no, primero pasa por `incq` que incrementa el valor de rax en 1,que gracias a `movq $0 , %rax` era cero, por lo que ahora es 1. 

[fig 3]
Para este caso nos encontramos dentro del ciclo, y tenemos “statements” adicionales, el código generado contempla que se está dentro de un ciclo y además genera el código de cada statement. Primero se evalúa si  rax es 0. Si es cierto el flag zero se activa. En la siguiente línea `jz` es un salto condicional que se ejecuta si el flag zero está activo. De ser así se salta a break (ver fig 5). Comenzado con la expresión interna de putc las siguientes 4 líneas se encargan de generar el valor de 65 + i. Las siguientes dos hacen el llamado a la función `putc`. Y el resto, menos `jmp` que realiza el salto de nuevo a loop para crear el ciclo, son el producto de `i = i + 1`. Vale la pena analizar que esas 4 instrucciones podrían ser sintetizadas en `addq $1, 8(%rbp)` o inclusive `incq 8(%rbp)` por lo que este código puede ser optimizado, un resultado como esos sería un producto de una sintaxis como `i++`, pero como la statement es distinto el compilador “toma el camino largo”. Finalmente el último segmento simplemente desaloja las variables y retorna a la función que lo llamó.

[fig 4]

# Conclusiones generales

# Conclusiones específicas

# Bibliografía

::: {#refs}
:::
