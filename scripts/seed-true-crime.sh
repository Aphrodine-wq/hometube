#!/usr/bin/env bash
set -euo pipefail

source_dir="${1:-/tmp/hometube-true-crime}"
target_dir="${2:-${HOME}/Videos/True-Crime}"
mkdir -p "$target_dir"

remux() {
  local input="$1"
  local output="$2"
  local title="$3"
  local creator="$4"
  local description="$5"
  local source_url="$6"

  ffmpeg -hide_banner -loglevel error -y \
    -i "$source_dir/$input" -map 0:v -map 0:a? -map 0:s? -c copy -movflags +faststart \
    -metadata title="$title" \
    -metadata artist="$creator" \
    -metadata description="$description" \
    -metadata comment="$source_url" \
    "$target_dir/$output"
}

remux "01-boosting.mp4" "Boosting is a Business.mp4" \
  "Boosting is a Business" "Golden State Film Productions" \
  "A police-supported examination of professional shoplifting, organized boosters, and the methods detectives used to stop retail crime." \
  "https://archive.org/details/0456BoostingIsABusiness01332620"

remux "02-investigation.mp4" "Elements of Investigation.mp4" \
  "Elements of Investigation" "Golden State Film Productions" \
  "A law-enforcement training film covering crime-scene evaluation, evidence preservation, witnesses, and criminal prosecution." \
  "https://archive.org/details/0456ElementsOfInvestigation01010403"

remux "03-adt.mp4" "When Every Minute Counts.mp4" \
  "When Every Minute Counts" "Jerry Fairbanks Productions" \
  "A documentary look at alarm dispatch, police response, and property-crime prevention in the analog era." \
  "https://archive.org/details/ADTWhenE1958"

remux "04-boy-in-court.mp4" "Boy in Court.mp4" \
  "Boy in Court" "Willard Pictures" \
  "A documentary account of the juvenile court system and its approach to young offenders and rehabilitation." \
  "https://archive.org/details/BoyinCou1940"

remux "05-kidnapping.mp4" "The Moskowitz Kidnapping.mp4" \
  "The Moskowitz Kidnapping" "Universal-International Newsreel" \
  "Newsreel coverage of realtor Leonard Moskowitz's kidnapping and rescue after sixty-four hours in captivity." \
  "https://archive.org/details/Kidnappi1950"

remux "06-police-dogs.mp4" "Police Dogs in Action.mp4" \
  "Police Dogs in Action" "Lee C. Garrison" \
  "A documentary profile of working police dogs, their training, handlers, and off-duty lives." \
  "https://archive.org/details/PoliceDogsInAction"

remux "07-profile-in-blue.mp4" "Profile in Blue.mp4" \
  "Profile in Blue" "Norman Rafsol" \
  "A close look at the Nassau County Police Department, its history, investigators, and specialized divisions." \
  "https://archive.org/details/ProfileInBlue"

remux "08-saint-paul-detectives.mp4" "Saint Paul Police Detectives.mp4" \
  "Saint Paul Police Detectives" "Saint Paul Police Department" \
  "A stop-motion documentary illustrating how Saint Paul detectives investigated crime and organized casework." \
  "https://archive.org/details/SaintPau1941"

remux "09-subject-narcotics.mp4" "Subject Narcotics.mp4" \
  "Subject: Narcotics" "Denis and Terry Sanders" \
  "A police-orientation film examining addiction, narcotics enforcement, and the social conditions of postwar Los Angeles." \
  "https://archive.org/details/SubjectN1951"

remux "10-police-department.mp4" "This Is Your Police Department.mp4" \
  "This Is Your Police Department" "Jam Handy Organization" \
  "A documentary tour of Detroit police operations, training, investigations, and public ceremonies." \
  "https://archive.org/details/ThisIsYo1951"

printf 'Seeded 10 public-domain videos in %s\n' "$target_dir"
