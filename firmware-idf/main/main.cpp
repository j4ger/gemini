#include <chrono>
#include <iostream>
#include <thread>

#include "esp_timer_cxx.hpp"

using namespace std;
using namespace idf;
using namespace idf::esp_timer;

extern "C" void app_main(void) {
  ESPTimer timer([]() { printf("Hello World!\n"); });

  timer.start_periodic(chrono::microseconds(200 * 1000));

  this_thread::sleep_for(std::chrono::milliseconds(10000));
}
