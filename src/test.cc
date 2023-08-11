
#include <stdio.h>

// #include "yankpass/include/test.h"
#include "../include/test.h"
// #include "include/test.h"

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <iostream>
#include <string>
#include <utility>

#include "firebase/auth.h"
#include "firebase/auth/user.h"
#include "firebase/firestore.h"
#include "firebase/util.h"

using namespace firebase::firestore;
using firebase::App;
using firebase::AppOptions;
using firebase::Future;
using firebase::InitResult;
using firebase::firestore::DocumentReference;
using firebase::firestore::Error;
using firebase::firestore::FieldValue;
using firebase::firestore::Firestore;

// no need to initialise or destroy unique_ptr
Store::Store() {}
Store::~Store() {}

void Store::drop() {}

std::unique_ptr<Store> create(const char *config_json) {
  auto conf = AppOptions::LoadFromJsonConfig(config_json, nullptr);
  auto app = App::Create(*conf);
  InitResult result;
  auto db = Firestore::GetInstance(app, &result);

  auto store = std::unique_ptr<Store>(new Store());
  store->app = std::unique_ptr<App>(app);
  store->db = std::shared_ptr<Firestore>(db);
  return store;
}

void Store::update_data(const char *obj_json) {
  this->db->Collection("users").Document("root").Set(
      {{"json", FieldValue::String(obj_json)}});
}

void myap(const std::string &config_str) {
  auto conf = AppOptions::LoadFromJsonConfig(config_str.c_str(), nullptr);
  auto app = App::Create(*conf);
  InitResult result;
  auto db = Firestore::GetInstance(app, &result);

  auto user_ref = db->Collection("users").Add({
      {"first", FieldValue::String("Ada")},
      {"last", FieldValue::String("Lovelace")},
      {"born", FieldValue::Integer(1815)},
  });

  user_ref.OnCompletion([](const Future<DocumentReference> &future) {
    if (future.error() == Error::kErrorOk) {
      std::cout << "DocumentSnapshot added with ID: " << future.result()->id()
                << std::endl;
    } else {
      std::cout << "Error adding document: " << future.error_message()
                << std::endl;
    }
  });
}