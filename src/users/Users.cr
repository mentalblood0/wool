require "yaml"

require "trove"
require "woollib/common"

module Wool
  class Users
    mserializable

    getter chest : Trove::Chest

    def add(u : User) : Id
      raise Exception.new "User #{u.to_json} already exists" if @chest.has_key! u.id, "type"
      @chest.set u.id, "", Wool.to_tj u
      u.id
    end

    def get(ui : Id) : User?
      Wool.from_tj User, (@chest.get ui).not_nil! rescue nil
    end

    def delete(ui : Id) : Id
      @chest.transaction do |tx|
        tx.delete ui
        tx.where({"pseudonym.user_id" => ui.string}) { |ii| tx.delete ii }
      end
      ui
    end

    def add(i : Pseudonym) : Id
      raise Exception.new "Pseudonym #{i.to_json} already exists" if @chest.has_key! i.id, "type"
      @chest.set i.id, "", Wool.to_tj i
      i.id
    end

    def get(s : Site, pseudonym : String) : User?
      ii = @chest.where({"pseudonym.site" => (String.from_json s.to_json),
                         "pseudonym.name" => pseudonym}).first rescue return nil
      i = Wool.from_tj Pseudonym, (@chest.get ii).not_nil!
      Wool.from_tj User, (@chest.get i.user_id).not_nil! rescue nil
    end

    def delete(i : Pseudonym) : Id
      @chest.delete i.id
      i.id
    end

    def push(ui : Id, c : Wool::Command) : UInt32
      @chest.push ui, "user.queue", [JSON.parse c.to_json]
    end
  end
end
