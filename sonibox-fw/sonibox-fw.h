/* Header file for the sonibox firmware.
 * All wiring variables should be defined here!
 */

#include "Arduino.h"

// Debug
const bool debug = true;
const bool debug_volume = false;

// Delays
const int button_delay = 500;  // ms

// Buttons
const int pin_play_button = 3;
const int pin_fwd_button = 4;
const int pin_bck_button = 5;

// Volume knob
const int pin_volume = A0;

// LEDs
const int pin_led_red = 22;
const int pin_led_green = 24;
const int pin_led_yellow = 26;

// MP3 player
#include "DFRobotDFPlayerMini.h"
#define Mp3Serial Serial1
DFRobotDFPlayerMini Mp3Player;
const uint8_t mp3_eq = DFPLAYER_EQ_NORMAL;
const uint8_t mp3_device = DFPLAYER_DEVICE_SD;
const int min_volume = 0;
const int max_volume = 30;

// Reader
#include <SPI.h>
#include <MFRC522.h>
const int pin_reader_rst = 9;
const int pin_reader_ss = 6;
const int pin_reader_irq = 2;
MFRC522 reader(pin_reader_ss, pin_reader_rst);

const int max_card_removal_reads = 10;

// Figures - Figure UID to folder number mapping for each figure
const int num_of_figures = 3;
const unsigned long figure_mapping[num_of_figures][2] = {
  {4294951236,  1},
  {21849, 2},
  {32108, 3}
};
