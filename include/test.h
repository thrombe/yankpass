
#pragma once
#include "firebase/firestore.h"
#include "firebase/util.h"
#include <memory>

#include "rust/cxx.h"
// #include "yankpass/src/main.rs.h"
#include "yankpass/src/bridge.rs.h"

// struct UpdateDataContext;

using firebase::App;
using firebase::firestore::Firestore;

using c_void = void;

class Store {
public:
  std::unique_ptr<App> app;
  std::shared_ptr<Firestore> db;

  Store();
  ~Store();

  void drop();

  void update_data(const char *, rust::Fn<void(void *ctx, const char *val)>,
                   void *);
  // void update_data(const char *);
};

std::unique_ptr<Store> create(const char *);
