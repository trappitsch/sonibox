/* Sonibox firmware
 * Wiring defined in sonibox-fw.h
 * Here we do the whole logic of the box
 */

#include "sonibox-fw.h"

// Status
bool is_play = false;  // music playing or not

int play_folder = 1;
int play_track = 1;

bool card_present = false;
unsigned long current_card_id = 0;
int card_removal_reads = 0;  // how many reads show that the card was removed?


int current_volume = 10;


void setup() {
  // anlog read
  analogReadResolution(12);  // 4096

  // Serial for printing
  Serial.begin(115200);
  if (debug) {
    Serial.println("Starting setup");
  }

  // LEDs
  pinMode(pin_led_red, OUTPUT);
  pinMode(pin_led_green, OUTPUT);
  pinMode(pin_led_yellow, OUTPUT);

  // Turn error LED on
  digitalWrite(pin_led_red, HIGH);

  // Buttons
  pinMode(pin_play_button, INPUT);
  pinMode(pin_fwd_button, INPUT);
  pinMode(pin_bck_button, INPUT);

  // initialize the MP3 player
  Mp3Serial.begin(9600);
  if (!Mp3Player.begin(Mp3Serial, true, true)) {
    Serial.println("MP3 Player setup issue (connection or SD card!).");
    while(true);
  }
  Mp3Player.setTimeOut(500);
  Mp3Player.volume(current_volume);
  Mp3Player.EQ(mp3_eq);
  Mp3Player.outputDevice(mp3_device);
  if (debug) {
    Serial.println("MP3 Player setup successful!");
  }

  // initialize the reader
  SPI.begin();
  reader.PCD_Init();
  delay(10);
  if (debug) {
    Serial.println("Reader initialized and ready...");
  }

  if (debug) {
    Serial.println("Setup successfully completed.");
  }
  // turn error led off
  digitalWrite(pin_led_red, LOW);
}

void loop() {
  if (digitalRead(pin_play_button) == LOW) {
    play_button_pressed();
  }
  if (digitalRead(pin_fwd_button) == LOW) {
    fwd_button_short_pressed();
  }
  if (digitalRead(pin_bck_button) == LOW) {
    bck_button_short_pressed();
  }

  adjust_volume();

  if (reader.PICC_IsNewCardPresent()) {
    card_present = true;
    unsigned long uid = get_card_id();
    if (uid != current_card_id && uid != 0) {
      current_card_id = uid;
      set_figure();
    }
  } else if (!reader.PICC_IsNewCardPresent() && !reader.PICC_ReadCardSerial()) {  // so there's no card
    if (card_present) {
      card_removed();
    }
  }
  delay(50);
}

// Count how long card was removed and if long enough, turn all off.
void card_removed() {
  if (card_removal_reads <= max_card_removal_reads) {  // avoid glitches in non-detection
      card_removal_reads += 1;
    } else {  // well, card is gone
      if (debug) {
        Serial.println("Card removed");
      }
      card_present = false;
      current_card_id = 0;

      if (is_play) {
        if (debug) {
          Serial.println("Card removed while playing, stop!");
        }
        Mp3Player.pause();
        is_play = false;
      }
    }
}

void set_figure() {
  // Set the play folder according to the figure that is on the box
  if (debug) {
      Serial.print("Current card ID: "); Serial.println(current_card_id);
  }
  play_folder = 0;
  for (int it = 0; it < num_of_figures; it++) {
    if (figure_mapping[it][0] == current_card_id) {
      play_folder = figure_mapping[it][1];
      break;
    }
  }
  if (debug) {
    Serial.print("Play folder number: "); Serial.println(play_folder);
  }
  // now play the music
  if (play_folder > 0) {
    is_play = true;
    Mp3Player.loopFolder(play_folder);
  }
}

unsigned long get_card_id(){
  if ( ! reader.PICC_ReadCardSerial()) { //Since a PICC placed get Serial and continue
    return 0;
  }
  unsigned long hex_num;
  hex_num =  reader.uid.uidByte[0] << 24;
  hex_num += reader.uid.uidByte[1] << 16;
  hex_num += reader.uid.uidByte[2] <<  8;
  hex_num += reader.uid.uidByte[3];
  // reader.PICC_HaltA(); // Stop reading
  return hex_num;
}

void adjust_volume() {
  // Todo: proper calcualtion of the volume according to the voltage divider
  int pot_value = analogReadMilliVolts(pin_volume);

  float tmp = float(pot_value) / vol_mv_max * max_volume;
  int new_volume = int(tmp);
  if (debug_volume) {
    Serial.print("Pot value: ");
    Serial.println(pot_value);
    Serial.print("Current volume: ");
    Serial.println(current_volume);
    Serial.print("tmp: ");
    Serial.println(tmp);
    Serial.println();
    delay(200);
  }
  int new_volume_int = int(new_volume);
  if (new_volume_int != current_volume) {
    if (debug) {
      Serial.print("Volume adjusted to ");
      Serial.println(new_volume_int);
    }
    Mp3Player.volume(new_volume_int);
    current_volume = new_volume_int;
  }
}

void play_button_pressed() {
  if (debug) {
    Serial.println("Play button pressed");
  }

  if (!card_present) {
    delay(button_delay);
    return;
  }

  is_play = !is_play;
  
  if (is_play) {
    Mp3Player.start();
    if (debug) {
      Serial.println("Mp3 Player starting");
    }
  } else {
    Mp3Player.pause();
    if (debug) {
      Serial.println("Mp3 Player paused");
    }
  }
  delay(button_delay);
}

void bck_button_long_pressed() {

}

void bck_button_short_pressed() {
  if (debug) {
    Serial.println("Back button short pressed.");
  }
  if (is_play) {
    Mp3Player.previous();
  }
  delay(button_delay);

}

void fwd_button_long_pressed() {

}

void fwd_button_short_pressed() {
  if (debug) {
    Serial.println("Forward button short pressed.");
  }
  if (is_play) {
    Mp3Player.next();
  }
  delay(button_delay);
}
