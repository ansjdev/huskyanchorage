# Plugin do SignalRGB

`HuskyAnchorage_Controller.js` é o plugin do SignalRGB para o Husky Anchorage
HTG200 (`320f:5016`). Ele faz duas coisas:

1. Manda as cores ao teclado pelo protocolo EVision V2, como qualquer plugin.
2. Espelha o mesmo quadro no keyglow por UDP, para a prévia do modo
   "Do SignalRGB" mostrar o que está acontecendo de verdade.

## Instalação

Copie o arquivo para a pasta de plugins do SignalRGB e reinicie o programa:

```
%USERPROFILE%\Documents\WhirlwindFX\Plugins\
```

No log do SignalRGB deve aparecer `[keyglow] espelho aberto em 127.0.0.1:12721`.

O espelho é opcional: se o módulo `@SignalRGB/udp` não existir na versão
instalada, o plugin registra uma linha no log e segue controlando o teclado
normalmente — só a prévia do keyglow fica num tom fixo.

## O protocolo do espelho

Um datagrama UDP por quadro para `127.0.0.1:12721`, little-endian:

| Bytes | Conteúdo |
|---|---|
| 2 | `0x4B 0x47` (`"KG"`) |
| 1 | versão do formato (1) |
| 2 | número de LEDs (u16) |
| n×3 | as cores, em `R, G, B` por LED |

Com 126 LEDs dá 383 bytes. Quem lê do outro lado é `src/signalrgb.rs`.

## Por que o keyglow não manda nada ao teclado nesse modo

Dois programas no modo direto ao mesmo tempo fazem o teclado piscar. Ao entrar
em "Do SignalRGB" o keyglow põe o firmware no modo em que ele não desenha nada
(`MODE_CUSTOM`, que apaga a luz) e para de enviar quadros: o teclado fica só
para o SignalRGB. Ao sair do modo, o efeito de fábrica escolhido volta.
