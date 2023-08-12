
#include "../include/test.h"

#include <stdio.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <iostream>
#include <string>
#include <utility>

#include "firebase/firestore.h"
#include "firebase/util.h"

#include "rust/cxx.h"

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

void Store::update_data(const char *obj_json,
                        rust::Fn<void(void *ctx, const char *val)> done,
                        void *ctx) {
  // void Store::update_data(const char *obj_json) {
  auto ref = this->db->Collection("users").Document("root");

  auto fut = ref.Set({{"json", FieldValue::String(obj_json)}});

  fut.OnCompletion([done, ctx](const Future<void> &future) mutable {
    if (future.error() == Error::kErrorOk) {
      (*done)(ctx, nullptr);
    } else {
      auto str = future.error_message();
      (*done)(ctx, str);
    }
  });
}

void Store::set_listener(
    rust::Fn<void(void *ctx, const char *json, const char *err)> callb,
    void *ctx) {
  auto docref = this->db->Collection("users").Document("root");
  docref.AddSnapshotListener([callb, ctx](const DocumentSnapshot &snapshot,
                                          Error error,
                                          const std::string &errmsg) {
    if (error == Error::kErrorOk) {
      if (snapshot.exists()) {
        auto json = snapshot.Get("json").string_value();
        (*callb)(ctx, json.c_str(), nullptr);
      } else {
        std::cout << "json does not exists" << std::endl;
      }
    } else {
      (*callb)(ctx, nullptr, errmsg.c_str());
    }
  });
}
