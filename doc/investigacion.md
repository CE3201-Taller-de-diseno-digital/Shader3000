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
bibliography: investigacion.bib
csl: /home/josfemova/UsefulRepos/styles/ieee.csl
nocite: |
  @crcc, @icld
output:
  pdf_document:
    fig_caption: true
...

\maketitle
\thispagestyle{empty}
\clearpage
\tableofcontents
\pagenumbering{roman}
\clearpage
\pagenumbering{arabic}
\setcounter{page}{1}

# Enlace a video de presentación

"Generación de Código: Investigación para el curso de CE3104 - Compiladores e Intérpretes, tema 4"

<https://youtu.be/4XGjQPLzzBo>


# Objetivos

- Explicar los conceptos básicos detrás de la etapa de generación de código en un compilador actual.
- Describir los pasos mínimos requeridos a la hora de generar código en un compilador.
- Explicar una implementación ad hoc para generación de código en tiempo de ejecución.
- Brindar un ejemplo conceptual para ilustrar el proceso de generación de código.

# Introducción

En la presente investigación se pretende ahondar en una de las etapas llevadas a cabo durante el proceso de compilación, conocida como generación de código. Para ello se comenzará con una explicación de los conceptos básicos a manejar para comprender a cabalidad esta etapa, a su vez se explicarán los pasos mínimos más comunes requeridos al generar código, esto con el fin de brindar una comprensión global del proceso de manera simplificada y teórica. Este segmento de la investigación se basa principalmente en “Introduction to Compilers and Language Design”[@icld].

Adicionalmente, valiéndonos del conocimiento en el primer segmento, se expondrá un procedimiento alternativo para la etapa de generación de código. Dicho procedimiento consiste en la generación de código en tiempo de ejecución, un método que pretende generar código especializado creado rápidamente de manera dinámica, mediante el uso de templates y código precompilado. Para esta sección se ha utilizado como base “Compiling for Runtime Code Generation”[@crcc].

Finalmente se procederá a describir un ejemplo conceptual completo para reforzar los conceptos y crear una mayor cercanía con los temas, principalmente, de la primera sección de la investigación.

# Generación de código

## Generación de código y diseño de lenguaje

La generación de código constituye la etapa final del proceso de compilación. En este punto es generado, a partir de una serie de procesos, el código en el lenguaje meta, por lo general lenguaje máquina. En el capítulo _Generación de código y diseño de lenguaje_ se desarrollan ejemplos enfocados en el código ensamblador X86-64 ya que la teoría es fácil de adaptar a cualquier otro lenguaje ensamblador y se aborda el proceso en varias etapas o elementos básicos del programa sin utilizar métodos muy complejos. Es decir, el programa resultante no es óptimo.
 
### Funciones de apoyo
 
Primero se deben generar algunas funciones de apoyo que responden a la necesidad de registros temporales, para seguidamente generar expresiones. Los registros que se reservan tienen un número r, un nombre y un estado, además, pueden ser utilizados para gestionar la pila, para argumentos de funciones o para valores temporales.
Las funciones abstraidas son `scratch_alloc`, `scratch_free` y `scratch_name`. La función `scratch_alloc` se utiliza para encontrar algún registro reservado disponible y retorna su número o un mensaje de error. `scratch_free` marca registros antes reservados como disponibles y `scratch_name` retorna el nombre de un registro según su número.
Seguidamente se deben escribir las funciones `label_create` y `label_name` para crear labels únicas y anónimas que indican ramas condicionales o los objetivos de los saltos. Con `label_create` se incrementa un contador global y se retorna su valor y con `label_name` se retorna el nombre en forma de string, por ejemplo, el string equivalente al label 5 es “.L5”.
Por último se necesita una función `symbol_codegen` para mapear los símbolos de un programa con los símbolos de código ensamblador. Esta función examina las posibilidades del símbolo y retorna un string con el fragmento de una instrucción que representa el cálculo de la dirección que se necesita para ese símbolo. En el caso de una variable global es simple, pues corresponde a un símbolo que en código objeto obtiene el mismos nombre que en el código fuente. En el caso de variables locales y parámetros de una función, debería retornar el cálculo de la dirección exacta que tiene la posición de esa variable o parámetro en el marco de llamada, una estructura que se encuentra en la pila.
 
### Generación de expresiones

La forma más básica de generar código ensamblador para una expresión es con un recorrido postorden del árbol abstracto de sintaxis o grafo dirigido acíclico (AST y DAG respectivamente por sus siglas en inglés) en donde cada nodo tiene una o varias instrucciones para mantener un control de los registros. Si se agrega otro campo a los nodos para guardar el número de registro que se obtiene con `scratch_alloc` y por cada nodo que se visita se emite una instrucción y se coloca el número del registro que contiene ese valor en el nuevo campo. Cuando ya no se necesita se llama a `scratch_free` para liberarlo. 
Para lograr esto se crea una función `expr_codegen` que se llama recursivamente con su hijo izquierdo y derecho, cada hijo va a generar un código y su resultado va a guardarse en un registro de resultado. Por cada nodo se va a generar código con esos resultados y se va a liberar los que no se utilicen. Para este proceso hay que considerar que algunos nodos requieren varias instrucciones, por ejemplo en x86 la instrucción `imul`, utilizada para multiplicar dos enteros, toma implícitamente uno de los argumentos en `%rax`, por lo cual no puede utilizarse cualquier otro registro en su lugar al realizar esta operación. El resultado se posiciona en `%rax` el desborde en `%rdx` (formando un producto de 128 bits), por lo que se deben ir moviendo los resultados a registros temporales. Otra consideración es la llamada de funciones con `call`, donde cada argumento es un árbol desbalanceado de expresiones. El generador de código debe evaluar cada nodo y, dependiendo de la convención de llamada, se debe realizar un `push` al stack o se copian los resultados de argumentos a registros específicos. Finalmente, `call` realiza el salto apropiado al punto de entrada de la función invocada. Cuando la función retorna, un segmento final llamado epílogo se encarga de colocar el valor de retorno (de existir) en `%rax` y restaurar registros preservados en concordancia con la convención de llamada.

### Generación de estructuras

Ahora que se pueden generar expresiones con la función `expr_codegen`, se podrían generar estructuras de código que utilicen dichas expresiones. Se puede crear una función `stmt_codegen` que va a contener el código para generar cada sentencia. Se pueden tener varios casos, si la declaración es de una variable local (`STMT_DECL` para el modelo descrito) entonces se llama a la función `decl_codegen`, si es una expresión (`STMT_EXPR`) se llama a `expr_codegen` para luego liberar el registro temporal que contiene el valor de la expresión con `scratch_free`, ya que este valor no se utiliza. Un ejemplo de esto último es una sentencia como `f(g())`. El valor de retorno de `g()` se utiliza, pero el de `f()`, de existir, se descarta. También debe haber un caso de retorno `STMT_RETURN` que evalúa la expresión, la mueve al `%rax` y luego hace un salto a una función que desenvuelve la pila y vuelve al punto de llamado. Se pueden crear casos para declaraciones condicionales como los `if` y `else` así como para generar ciclos. En el caso de un `print`, al ser una función imperativa con comportamiento que varía según el tipo de expresión que se quiere imprimir, se debe generar una función diferente para cada tipo.
 
### Expresiones condicionales

Las expresiones condicionales (mayor que, menor que, igual, etc) comparan dos valores y retornan un booleano. Usualmente aparecen en las funciones de control de flujo de expresiones pero pueden ser utilizadas como valores simples. Para este propósito, el autor escoge generar estructuras condicionalese comparan las dos expresiones y construyen el resultado. Para diferentes operadores se utilizan diferentes saltos al lugar indicado y con algunas modificaciones se podría utilizar para implementar el operador condicional ternario. Si se utiliza este método para un `if` en el lenguaje utilizado habrían dos estructuras condicionales en ensamblador, la primera para computar la condición y ponerla en un registro y la segunda para comparar los resultados y saltar a el caso verdadero o el falso.

### Generación de declaraciones 

Para emitir el programa completo se debe recorrer cada declaración de código o datos y emitir su estructura. Las declaraciones de datos globales emiten una etiqueta junto con indicador que reserva el espacio necesario y un inicializador, en caso de ser necesario, que por ser una variable global debe ser una constante. Es posible extender este concepto a inicializadores arbitrarios, como es el caso de los constructores globales en C++, pero esto complica significativamente el proceso y se omite.

Para las declaraciones de variables locales se asume que el espacio fue reservado por el prólogo: de manera análoga al epílogo, este segmento se ubica al inicio de la función y se encarga de crear un marco en la pila, durante lo cual se delimita memoria para locales. Por ello, no se necesita manipular la distribución de la pila en lo posterior con tal de leer y escribir locales. Nótese que una definición de variable tiene en realidad dos efectos en la generación de código, específicamente la reservación de memoria en el marco y la evaluación de la expresión inicializadora.

Para generar declaraciones de funciones se debe emitir una etiqueta con el nombre de la función seguido por el prólogo. El prólogo debe tomar los parámetros y crear el espacio respectivo para el marco en la pila. Luego del prólogo se emite el cuerpo de la función, el cual es una secuencia de sentencias, y por último el epílogo. El diseño propuesto indica que el epílogo debe contener una etiqueta única para que las expresiones de retorno puedan saltar fácilmente, aunque existen alternativas.

### Ejemplo de generación de código

Para ilustrar el proceso de generación de código se analizará un ejemplo sencillo presentando el significado del código traducido así como su origen y el proceso que lo genera. Sea el siguiente código fuente para un lenguaje fuente propuesto:

```python
i = 0
while i < 10:
    putc(65 + i)
    i = i + 1
```

Dada la arquitectura x86-64, así como la teoría descrita con anterioridad, un posible listado emitido para el fragmento anterior podría ser el siguiente:

```asm
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
        leaq    8(%rsp), %rsp      # deallocate stack space for "i"
        ret                        # return to caller
```

Considérese la primera sección de este listado, a como muestra la Figura 1.

![\label{fig1} Traducción de una asignación](imgs/fig1.png){width=60%}

Las primeras tres instrucciones constituyen el prólogo de la función, y la cuarta es la inicialización de `i`. Nótese que el prólogo reserva memoria para las locales que se utilizarán durante la ejecución de la función y ajusta tanto stack pointer (`%rsp`) como frame pointer (`%rbp`) para formar un nuevo marco de llamada. `i = 0` es una declaración, por lo que para generar el código en x86 se utiliza el procedimiento anteriormente especificado para la generacion de declaraciones e inicializadores. La cuarta instrucción guarda un cero en la posición de memoria que corresponde a `i`. Nótese que un mecanismo de generación de código más óptimo evitaría esta última instrucción, ya que por consecuencia de `pushq $0` ya existe un valor de cero en esa posición.

A continuación se comienza y mantiene un bucle, ver Figura 2.

![\label{fig2} Traducción de sentencia while](imgs/fig2.png){width=60%}

Para generar código se utilizan los mismos procedimientos descritos en “Generación de estructuras”. Para este caso, únicamente se analiza la declaración del `while` y su condición. Las primeras tres instrucciones se utilizan para evaluar la condición. `cmpq` resta sus dos operandos (el segundo menos el primero, debido a la inversión de orden de operandos presente en sintaxis AT&T), altera banderas de `RFLAGS` según el resultaod de la resta y luego descarta el resultado de la resta. Estas banderas incluyen tales hechos como que el resultado sea cero o distinto de cero, que la interpretación en complemento a dos del resultado sea positiva o negativa, entre otras. `jnl` ("jump if not less") es un salto condicional que se da si y solo si las banderas adecuadas que resultan tras una operación `cmp b, a` donde `!(a < b)`. En este caso, ello quiere decir que el salto ocurre si y solo si `i >= 10`, la condición opuesta a la condición fuente del bucle. Si la condición de salto no se cumple, `jnl` se comporta igual que `nop` (no realiza ninguna acción). La Figura 3 muestra las consecuencias de un salto a `loop_3_less`. Si no hay salto, primero pasa por una instrucción `incq` que incrementa en 1 el valor de `%rax`, que gracias a `movq $0 , %rax` era cero, por lo que pasaría a ser 1 si y solo si la condición de salto no se cumple (es decir, si la condición del bucle sí se cumple). En otras palabras, al momento de llegar a `loop_3_less` el registro `%rax` contiene un booleano con el resultado de evaluar `i < 10`.

![\label{fig3} Traducción del cuerpo del ciclo](imgs/fig3.png){width=60%}

Para este caso nos encontramos dentro del ciclo, y tenemos las expresiones internas del ciclo. El código generado contempla que se está dentro de un ciclo y además genera el código de cada expresión. Primero, se evalúa si `%rax` es 0. Si es cierto entonces `ZF` ("zero flag") se activa en `RFLAGS`. En la siguiente línea `jz` es un salto condicional que se toma si y solo si `ZF` está activa. De ser así, se salta a `loop_2_break` (ver Figura 4). Comenzando con la expresión interna de llamada a `putc`, las siguientes 4 líneas se encargan de generar el valor de `65 + i`. Las siguientes dos hacen el llamado a la función `putc` con posiciones apropiadas de cada argumento. El resto (con excepción de `jmp`, que realiza el salto de nuevo a loop para crear el ciclo), son el producto de la sentencia `i = i + 1`. Vale la pena analizar que esas 4 instrucciones podrían ser sintetizadas en `addq $1, 8(%rbp)` o inclusive `incq 8(%rbp)`, lo que sugiere que este código puede ser optimizado (tal como se mecionaba al comienzo de la investigación). Un resultado como esos sería un producto de una sintaxis como `i++`, pero como la expresión es distinta el compilador “toma el camino largo”.

![\label{fig4} Traducción completa del ejemplo](imgs/fig4.png){width=30%}

Finalmente, el último segmento de tres instrucciones es el epílogo, el cual desaloja las variables locales, destruye el marco y retorna a la función que llamó.

## Generación de código en tiempo de ejecución

El artículo cubre la técnica de _Generación de código en tiempo de ejecución_ o RTCG por sus siglas en inglés. La última es una técnica de procesamiento de código que permite la generación dinámica de código compilado durante el tiempo de ejecución mismo. La técnica de compilado en tiempo de ejecución debe hacer un balance entre dos factores fundamentales: tiempo de compilación y optimización de código.

Se cubre este tema usando como ejemplo un lenguaje desarrollado por los autores llamado Cyclone, el cual es estáticamente tipado. Este lenguaje se transpila a una variante tipada de lenguaje ensamblador. Se menciona que durante el proceso de transpilado, anteriormente, para el código que era generado en tiempo de ejecución no tendría optimización alguna, lo que aumentaría el costo en recursos computacionales, y podría contrarrestar la ventaja de la generación de código de forma dinámica.

Se mencionan varios acercamientos para poder conseguir generación de código en tiempo de ejecución, entre ellas un modelo basado en templates sin optimizaciones, otro con manipulación de strings que son posteriormente compilados en tiempo de ejecución por un compilador convencional por lo tanto asegurando una optimización de código adecuada. Los autores deciden utilizar un tercer acercamiento, el cual consiste en analizar el flujo de datos de las funciones de generación de código de forma que se pueda obtener un grafo que caracteriza de forma general los posibles productos de la función generadora, esto permite realizar optimizaciones sobre los elementos de la función producto.

Para guiar en el tema, los autores introducen su lenguaje Mini-Cyclone, una variante reducida del lenguaje cyclone que se mencionó anteriormente. Este lenguaje no se compila directamente a lenguaje máquina, sino que su compilado es a un lenguaje intermedio que los autores llaman CIR (Cyclone Intermediate Representation). Este lenguaje mencionan es relativamnete estándar, un IR de bajo nivel y basado en bloques. Para comprender mejor el árticulo es escencial notar en especial algunos componentes del lenguaje que son fundamentales:

- `codegen`: Una expresión `codegen(...)` inicia la construcción de código en tiempo de ejecución a partir de plantillas.
- `cut`: Debe ocurrir dentro de un bloque que a su vez se encuentra dentro de `codegen`. `cut` retoma el contexto del "padre" o función generadora (aquella que ejecuta `codegen()`), lo cual le permite tomar decisiones arbitrarias acerca del código emitido.
- `splice`: Debe ocurrir dentro de un `cut`, y de manera análoga retoma el contexto de la función generada o "hija". Como se ejecuta dentro de un `cut`, un segmento generado por `splice` ya no se define estáticamente, sino que a criterio dinámico del bloque que se encuentra dentro de `cut`.
- `fill`: Se coloca en lugar de una expresión cualquiera para indicar un "hueco": una posición en el código IR plantilla que será sustituido por un valor calculado durante la generación en tiempo de ejecución, logrando así la especialización dinámica de funciones.

El resto del trabajo, así como la descripción aquí presentada, trata de la reducción de la forma fuente de código Mini-Cyclone a CIR, buscando la conservación de las propiedades de especialización dinámica de plantillas y la accesibilidad a etapas de optimización.

### Traducción de constructos que participan en la generación de código

Para aspectos de traducción a CIR se define la noción de un entorno de generación. Cada función de primer nivel en Mini-Cyclone se asocia a uno de estos entornos. Cada entorno dispone de una secuencia de entornos padres e hijos. Los entornos hijos resultan de la anidación de constructos estructurales (como por ejemplo condicionales y bucles) y de la generación de código a través de expresiones `codegen()`. El proceso de generación se define en términos de un entorno actual, el cual puede "ascender" o "descender" en la cadena de padres e hijos en respuesta a las operaciones `cut`, `splice` y `fill`.

En la implementación sugerida, el cambio entre entornos por `cut` y `splice` resulta siempre en un quiebre de plantilla. Es decir, subsecuente código generado se agregará a una nueva plantilla, tal que las funciones eventualmente generadas son la concatenación de distintas instanciaciones de estas plantillas. Cada una de las cuatro operaciones fundamentales de generación resultan en distintos fragmentos de CIR emitidos para distintos entornos relativos. La concatenación de plantillas es un proceso mayormente libre de complicaciones, con la excepción de saltos entre plantilla, siendo un claro ejemplo presentado por los autores una situación del tipo `while(expr) { cut { ... } }`. El lenguaje CIR presenta un constructo de salto especial y una instrucción de sustitución de huecos de salto para solventar este problema.

- `codegen` reserva memoria para construir una nueva función generada (desde el contexto padre) y entra en su entorno.
- `cut` corta la plantilla hija actual y retoma el entorno de la función generadora.
- `splice` funciona de manera muy parecida a `codegen`, pero indica la concatenación de una nueva plantilla a una función generada ya existente, en vez de crear una nueva como hace `codegen`.
- `fill` resulta en acciones para tanto el entorno padre como el hijo. En el padre, emite código para evaluar la expresión indicada y colocar su resultado en la posición del hueco asociado. Del lado del hijo, emite código CIR que indica la presencia del hueco.

Nótese como la analogía con concatenación de cadenas textuales se sostiene. En un sentido reducido, la generación de código en tiempo de ejecución para el lenguaje Mini-Cyclone se vuelve una extensión a las ideas anteriormente presentadas para generación de código en tiempo de compilación, en vez de una modificación. Los constructos presentados extienden las mismas para abarcar la identificación y especialización de plantillas, pero los métodos para todas las demás operaciones se mantienen. Así, resulta efectivamente equivalente la traducción de declaraciones, bucles, condicionales, secuencias de sentencias y expresiones no generadoras.

### Optimización y uso de CFG

La presencia de código generado (o más bien, especializado) en tiempo de ejecución otorga dos posibles focos para las etapas de optimización: optimización tradicional durante compilación de fragmentos y bloques, y optimización especializada durante la unificación de fragmentos con huecos sustituidos para formar una función generada.

Las etapas de optimización de un compilador tradicional dependen de un grafo de control de flujo (CFG por sus siglas en inglés) para determinar dependencias entre variables, expresiones, llamadas, constantes, scopes y tiempos de vida de datos. Sin embargo, la presencia de bloques que no son siempre código estáticamente definido, dada la existencia de plantillas y huecos, así como de la necesidad de modificar código de manera parcial en tiempo de ejecución, complica significativamente la teoría detrás de la construcción de este grafo. Con ello se agrega la llamada etapa de análisis bajo la propuesta de los autores, para luego pasar a la más convencional etapa de transformación.

#### Etapa de análisis

La estrategia escogida por los autores es la de optimizar en tiempo de compilación cada fragmento de plantilla, con tal de que la concatenación en tiempo de ejecución sea computacionalmente trivial. Para saltos y llamadas dentro de un mismo entorno (también bloque o plantilla según el nivel de abstracción), la determinación de vértices y aristas del grafo de control de flujo es no presenta cambios con respecto a lo que usualmente ocurre para un lenguaje sin generación de código en tiempo de ejecución. Sin embargo, los saltos entre plantillas presentan el problema de que no hay forma de determinar en el caso general durante compilación el destino de la arista. La solución propuesta consiste en la formulación de una serie de ecuaciones que se derivan a partir del hecho de que todo entorno puede tener a lo mucho un padre en el caso de Mini-Cyclone. El grafo es la solución a estas ecuaciones.

#### Etapa de transformación

Las optimizaciones aplicables sobre IR con huecos son mayormente idénticas a las que se conocen de la teoría clásica para optimización en tiempo de compilación. Los autores mencionan de manera breve que deben considerarse a los huecos como cajas negras, así como consideraciones de cuidado menor al referirse a las mismas (no pueden reducirse, simplificarse o unificarse a lo largo de plantillas distintas, ya que se corre el riesgo de alterar el significado del programa).

# Conclusiones específicas

## Sobre el artículo de generación de código

- Antes de poder generar expresiones, hace falta ser capaces de alojar, nombrar y liberar registros en el procesador. 
- Es posible generar código para expresiones si son representadas como estructuras AST o DAG, y posteriormente estas mismas estructuras son recorridas en postorden.
- En ocasiones una única expresión puede generar multiples líneas de código ensamblador, esto debido a alojos y desalojos de registros o casos especiales como es el `imul` de x86.
- Es posible implementar estructuras bifurcantes y anidadas, como condicionales y ciclos, utilizando únicamente saltos condicionados entre distintos fragmentos de código secuencial en ensamblador.

## Sobre compilación de para lenguajes que generan código en tiempo de ejecución

- La generación de código en tiempo de ejecución es, con algunas diferencias relativamente menores, el mismo proceso que la generación en tiempo de compilación.
- La etapa de optimización en un lenguaje que debe generar código de manera dinámica en tiempo de ejecución es un poco más compleja que para un lenguaje que no tiene esta funcionalidad.
- La compilación de un lenguaje para que el mismo pueda generar código en tiempo de ejecución sigue siendo un área en investigación que tiene espacio para mejorar el proceso de compilado de forma que el código se encuentre lo más optimizado posble.
- Al compilar un código que genera funcionalidad de manera dinámica hay que tener en cuenta los impactos en el desempeño del sistema que esta generación dinámica pueda tener, y desarrollar las contramedidas necesarias para lidiar con estos impactos o minimizarlos.

# Conclusiones generales

- Las etapas de optimización antes y alrededor de la generación de código son fundamentales para asegurar la eficiencia.
- La generación de código es un proceso que se apoya en el uso de grafos y otras estructuras de datos para tratar de linealizar la estructura lógica de un programa, de manera que el mismo pueda ser traducido de manera efectiva a lenguaje máquina.
- Las estapas de generación de código involucran procedimientos cuya teoría requiere de un entendimiento adecuado de conceptos algo complejos, por lo que es importante tener una base sólida en conceptos matemáticos y de programación de forma que el entendimiento de estos conceptos se facilite.
- Generación de código sin optimización es un buen ejercicio para comprender los principios detrás de este proceso, pero en aplicaciones reales no se puede prescindir de la etapa de optimización por los impactos que esto tendría en el desempeño del programa compilado.

$~~~~~$
$~~~~~$

# Bibliografía

::: {#refs}
:::
