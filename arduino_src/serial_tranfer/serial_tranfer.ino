/*
   Управление лентой на WS2812 с компьютера + динамическая яркость
   Создано не знаю кем, допилил и перевёл AlexGyver http://alexgyver.ru/
   2017

   Допилил под свой проект lmaxyz.
   2024
*/
//----------------------НАСТРОЙКИ-----------------------
#define NUM_LEDS 92          // число светодиодов в ленте
#define DI_PIN 5            // пин, к которому подключена лента
#define OFF_TIME 10          // время (секунд), через которое лента выключится при пропадаании сигнала
#define CURRENT_LIMIT 2000   // лимит по току в миллиамперах, автоматически управляет яркостью (пожалей свой блок питания!) 0 - выключить лимит

#define START_FLASHES 1      // проверка цветов при запуске (1 - включить, 0 - выключить)

//#define AUTO_BRIGHT 1        // автоматическая подстройка яркости от уровня внешнего освещения (1 - включить, 0 - выключить)
#define MAX_BRIGHT 200       // максимальная яркость (0 - 255)
#define MIN_BRIGHT 50        // минимальная яркость (0 - 255)
//#define BRIGHT_CONSTANT 500  // константа усиления от внешнего света (0 - 1023)
// чем МЕНЬШЕ константа, тем "резче" будет прибавляться яркость
#define COEF 0.9             // коэффициент фильтра (0.0 - 1.0), чем больше - тем медленнее меняется яркость
//----------------------НАСТРОЙКИ-----------------------

int new_bright, new_bright_f;
unsigned long bright_timer, off_timer;

#define serialRate 250000  // скорость связи с ПК
uint8_t prefix[] = {'A', 'd', 'a'}, hi, lo, chk, i;  // кодовое слово Ada для связи
#include <FastLED.h>
CRGB leds[NUM_LEDS];  // создаём ленту
boolean led_state = true;  // флаг состояния ленты

void setup()
{
  FastLED.addLeds<WS2812, DI_PIN, GRB>(leds, NUM_LEDS);  // инициализация светодиодов
  if (CURRENT_LIMIT > 0) FastLED.setMaxPowerInVoltsAndMilliamps(5, CURRENT_LIMIT);

  // вспышки красным синим и зелёным при запуске (можно отключить)
  if (START_FLASHES) {
    LEDS.showColor(CRGB(255, 0, 0));
    delay(500);
    LEDS.showColor(CRGB(0, 255, 0));
    delay(500);
    LEDS.showColor(CRGB(0, 0, 255));
    delay(500);
    LEDS.showColor(CRGB(0, 0, 0));
  }

  Serial.begin(serialRate);
  //Serial.print("Ada\n");     // Связаться с компом
}

void check_connection() {
  if (led_state) {
    if (millis() - off_timer > (OFF_TIME * 1000)) {
      led_state = false;
      FastLED.clear();
      FastLED.show();
    }
  }
}

void loop() {
  if (!led_state) led_state = true;
  off_timer = millis();  

  for (i = 0; i < sizeof prefix; ++i) {
waitLoop: while (!Serial.available()) check_connection();;
    if (prefix[i] == Serial.read()) {
      continue;
    }
    i = 0;
    goto waitLoop;
  }
  Serial.print("Ok\n");

  // while (!Serial.available()) check_connection();;
  // hi = Serial.read();
  // while (!Serial.available()) check_connection();;
  // lo = Serial.read();
  // while (!Serial.available()) check_connection();;
  // chk = Serial.read();
  // if (chk != (hi ^ lo ^ 0x55))
  // {
  //   i = 0;
  //   goto waitLoop;
  // }
  byte r1, g1, b1;
  memset(leds, 0, NUM_LEDS * sizeof(struct CRGB));
  for (int i = 0; i < NUM_LEDS; i++) {
    byte r, g, b;
    
    // читаем данные для каждого цвета
    while (!Serial.available()) check_connection();
    r=r1 = Serial.read();
    while (!Serial.available()) check_connection();
    g=g1 = Serial.read();
    while (!Serial.available()) check_connection();
    b=b1 = Serial.read();
  
    leds[i].r = r;
    leds[i].g = g;
    leds[i].b = b;
  }
  Serial.print(r1);
  Serial.print(g1);
  Serial.print(b1);
  // Serial.print("Ada\n");
  FastLED.show();  // записываем цвета в ленту
}